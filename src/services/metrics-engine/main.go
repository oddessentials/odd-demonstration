package main

import (
	"context"
	"encoding/json"
	"log"
	"os"
	"time"


	amqp "github.com/rabbitmq/amqp091-go"
	"github.com/redis/go-redis/v9"
	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"
)

type EventEnvelope struct {
	ContractVersion string                 `json:"contractVersion"`
	EventType       string                 `json:"eventType"`
	EventID         string                 `json:"eventId"`
	OccurredAt      string                 `json:"occurredAt"`
	Payload         map[string]interface{} `json:"payload"`
}

func getEnv(key, fallback string) string {
	if value, ok := os.LookupEnv(key); ok {
		return value
	}
	return fallback
}

func main() {
	log.Println("Metrics Engine starting...")

	rabbitURL := getEnv("RABBITMQ_URL", "amqp://guest:guest@rabbitmq:5672")
	redisURL := getEnv("REDIS_URL", "redis:6379")
	mongoURL := getEnv("MONGO_URL", "mongodb://admin:password123@mongodb:27017")

	// Connect to Redis
	rdb := redis.NewClient(&redis.Options{
		Addr: redisURL,
	})
	ctx := context.Background()

	// Test Redis connection
	for {
		_, err := rdb.Ping(ctx).Result()
		if err == nil {
			log.Println("Connected to Redis")
			break
		}
		log.Printf("Waiting for Redis... %v", err)
		time.Sleep(5 * time.Second)
	}

	// Connect to MongoDB
	var err error
	var mongoClient *mongo.Client
	mongoClient, err = mongo.Connect(ctx, options.Client().ApplyURI(mongoURL))
	if err != nil {
		log.Fatalf("Failed to create MongoDB client: %v", err)
	}
	for {
		err = mongoClient.Ping(ctx, nil)
		if err == nil {
			log.Println("Connected to MongoDB")
			break
		}
		log.Printf("Waiting for MongoDB... %v", err)
		time.Sleep(5 * time.Second)
	}
	db := mongoClient.Database("observatory")
	eventsColl := db.Collection("job_events")

	// Connect to RabbitMQ
	var conn *amqp.Connection
	for {
		conn, err = amqp.Dial(rabbitURL)
		if err == nil {
			break
		}
		log.Printf("Waiting for RabbitMQ... %v", err)
		time.Sleep(5 * time.Second)
	}
	defer conn.Close()
	log.Println("Connected to RabbitMQ")

	ch, err := conn.Channel()
	if err != nil {
		log.Fatalf("Failed to open channel: %v", err)
	}
	defer ch.Close()

	// Declare queue
	q, err := ch.QueueDeclare("jobs.completed", true, false, false, false, nil)
	if err != nil {
		log.Fatalf("Failed to declare queue: %v", err)
	}

	msgs, err := ch.Consume(q.Name, "", false, false, false, false, nil)
	if err != nil {
		log.Fatalf("Failed to register consumer: %v", err)
	}

	log.Println("Waiting for messages...")

	for msg := range msgs {
		var event EventEnvelope
		if err := json.Unmarshal(msg.Body, &event); err != nil {
			log.Printf("Error parsing message: %v", err)
			msg.Nack(false, false)
			continue
		}

		log.Printf("Received event: %s (%s)", event.EventID, event.EventType)

		// Update Redis counters
		switch event.EventType {
		case "job.completed":
			rdb.Incr(ctx, "metrics:jobs:completed")
			rdb.Incr(ctx, "metrics:jobs:total")
		case "job.failed":
			rdb.Incr(ctx, "metrics:jobs:failed")
			rdb.Incr(ctx, "metrics:jobs:total")
		}

		// Store last event time
		rdb.Set(ctx, "metrics:last_event_time", time.Now().Format(time.RFC3339), 0)

		// Store raw event in MongoDB
		_, err = eventsColl.InsertOne(ctx, event)
		if err != nil {
			log.Printf("Error storing event in MongoDB: %v", err)
		}

		msg.Ack(false)
		log.Printf("Processed event: %s", event.EventID)
	}
}
