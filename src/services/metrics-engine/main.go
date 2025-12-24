package main

import (
	"context"
	"encoding/json"
	"log"
	"os"
	"regexp"
	"strings"
	"time"

	"github.com/distributed-task-observatory/metrics-engine/validator"
	amqp "github.com/rabbitmq/amqp091-go"
	"github.com/redis/go-redis/v9"
	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"
)

// ServiceVersion is read from VERSION file at startup
var ServiceVersion string

// readVersion reads and validates the VERSION file
func readVersion() string {
	data, err := os.ReadFile("VERSION")
	if err != nil {
		log.Fatalf("FATAL: Failed to read VERSION file: %v", err)
	}
	version := strings.TrimSpace(string(data))

	// Validate SemVer format
	semverRegex := regexp.MustCompile(`^\d+\.\d+\.\d+$`)
	if !semverRegex.MatchString(version) {
		log.Fatalf("FATAL: Invalid SemVer format in VERSION file: %s", version)
	}

	return version
}

type EventEnvelope struct {
	ContractVersion string                 `json:"contractVersion"`
	EventType       string                 `json:"eventType"`
	EventID         string                 `json:"eventId"`
	OccurredAt      string                 `json:"occurredAt"`
	CorrelationID   string                 `json:"correlationId"`
	Payload         map[string]interface{} `json:"payload"`
}

// DLQMessage represents a message sent to the dead-letter queue
type DLQMessage struct {
	OriginalEvent   json.RawMessage `json:"original_event"`
	ValidationError string          `json:"validation_error"`
	RejectedAt      string          `json:"rejected_at"`
	CorrelationID   string          `json:"correlation_id"`
	Service         string          `json:"service"`
}

const (
	DLQName = "jobs.failed.validation"
)

func getEnv(key, fallback string) string {
	if value, ok := os.LookupEnv(key); ok {
		return value
	}
	return fallback
}

func main() {
	// Read and validate version at startup
	ServiceVersion = readVersion()
	log.Printf("Metrics Engine version %s starting...", ServiceVersion)

	rabbitURL := getEnv("RABBITMQ_URL", "amqp://guest:guest@rabbitmq:5672")
	redisURL := getEnv("REDIS_URL", "redis:6379")
	mongoURL := getEnv("MONGO_URL", "mongodb://admin:password123@mongodb:27017")

	// Initialize validator
	schemaValidator, err := validator.NewValidator()
	if err != nil {
		log.Fatalf("Failed to initialize schema validator: %v", err)
	}
	log.Println("Schema validator initialized")

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

	// Declare queues
	q, err := ch.QueueDeclare("jobs.completed", true, false, false, false, nil)
	if err != nil {
		log.Fatalf("Failed to declare queue: %v", err)
	}

	// Declare DLQ for validation failures
	_, err = ch.QueueDeclare(DLQName, true, false, false, false, nil)
	if err != nil {
		log.Fatalf("Failed to declare DLQ: %v", err)
	}

	msgs, err := ch.Consume(q.Name, "", false, false, false, false, nil)
	if err != nil {
		log.Fatalf("Failed to register consumer: %v", err)
	}

	log.Printf("Waiting for messages... DLQ enabled: %s", DLQName)

	for msg := range msgs {
		correlationID := validator.GetCorrelationID(msg.Body)

		// Validate message against schemas
		result := schemaValidator.ValidateMessage(msg.Body)
		if !result.Valid {
			errorMsg := formatValidationErrors(result.Errors)
			log.Printf("[%s] VALIDATION FAILED: %s", correlationID, errorMsg)

			// Publish to DLQ
			dlqMessage := DLQMessage{
				OriginalEvent:   msg.Body,
				ValidationError: errorMsg,
				RejectedAt:      time.Now().UTC().Format(time.RFC3339),
				CorrelationID:   correlationID,
				Service:         "metrics-engine",
			}
			dlqBytes, _ := json.Marshal(dlqMessage)

			err := ch.Publish("", DLQName, false, false, amqp.Publishing{
				DeliveryMode: amqp.Persistent,
				ContentType:  "application/json",
				Body:         dlqBytes,
			})
			if err != nil {
				log.Printf("[%s] Failed to publish to DLQ: %v", correlationID, err)
			}

			msg.Nack(false, false)
			continue
		}

		var event EventEnvelope
		if err := json.Unmarshal(msg.Body, &event); err != nil {
			log.Printf("[%s] Error parsing message: %v", correlationID, err)
			msg.Nack(false, false)
			continue
		}

		log.Printf("[%s] Received event: %s (%s)", correlationID, event.EventID, event.EventType)

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
			log.Printf("[%s] Error storing event in MongoDB: %v", correlationID, err)
		}

		msg.Ack(false)
		log.Printf("[%s] Processed event: %s", correlationID, event.EventID)
	}
}

func formatValidationErrors(errors []validator.ValidationError) string {
	if len(errors) == 0 {
		return "unknown error"
	}
	result := ""
	for i, e := range errors {
		if i > 0 {
			result += "; "
		}
		result += e.Field + ": " + e.Message
	}
	return result
}
