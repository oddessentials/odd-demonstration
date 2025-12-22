package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"time"

	_ "github.com/lib/pq"
	"github.com/redis/go-redis/v9"
)

type StatsResponse struct {
	TotalJobs     int64  `json:"totalJobs"`
	CompletedJobs int64  `json:"completedJobs"`
	FailedJobs    int64  `json:"failedJobs"`
	LastEventTime string `json:"lastEventTime"`
}

type Job struct {
	ID        string `json:"id"`
	Type      string `json:"type"`
	Status    string `json:"status"`
	CreatedAt string `json:"createdAt"`
}

var rdb *redis.Client
var db *sql.DB
var ctx = context.Background()

func getEnv(key, fallback string) string {
	if value, ok := os.LookupEnv(key); ok {
		return value
	}
	return fallback
}

func healthHandler(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	w.Write([]byte("OK"))
}

func statsHandler(w http.ResponseWriter, r *http.Request) {
	total, _ := rdb.Get(ctx, "metrics:jobs:total").Int64()
	completed, _ := rdb.Get(ctx, "metrics:jobs:completed").Int64()
	failed, _ := rdb.Get(ctx, "metrics:jobs:failed").Int64()
	lastEvent, _ := rdb.Get(ctx, "metrics:last_event_time").Result()

	stats := StatsResponse{
		TotalJobs:     total,
		CompletedJobs: completed,
		FailedJobs:    failed,
		LastEventTime: lastEvent,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

func recentJobsHandler(w http.ResponseWriter, r *http.Request) {
	rows, err := db.Query("SELECT id, type, status, created_at FROM jobs ORDER BY created_at DESC LIMIT 10")
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	var jobs []Job
	for rows.Next() {
		var job Job
		var createdAt time.Time
		if err := rows.Scan(&job.ID, &job.Type, &job.Status, &createdAt); err != nil {
			continue
		}
		job.CreatedAt = createdAt.Format(time.RFC3339)
		jobs = append(jobs, job)
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(jobs)
}

func corsMiddleware(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		
		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}
		
		next(w, r)
	}
}

func main() {
	log.Println("Read Model API starting...")

	redisURL := getEnv("REDIS_URL", "redis:6379")
	postgresURL := getEnv("POSTGRES_URL", "postgres://admin:password123@postgres:5432/task_db?sslmode=disable")

	// Connect to Redis
	rdb = redis.NewClient(&redis.Options{
		Addr: redisURL,
	})
	for {
		_, err := rdb.Ping(ctx).Result()
		if err == nil {
			log.Println("Connected to Redis")
			break
		}
		log.Printf("Waiting for Redis... %v", err)
		time.Sleep(5 * time.Second)
	}

	// Connect to PostgreSQL
	var err error
	for {
		db, err = sql.Open("postgres", postgresURL)
		if err == nil {
			if err = db.Ping(); err == nil {
				log.Println("Connected to PostgreSQL")
				break
			}
		}
		log.Printf("Waiting for PostgreSQL... %v", err)
		time.Sleep(5 * time.Second)
	}

	http.HandleFunc("/health", corsMiddleware(healthHandler))
	http.HandleFunc("/stats", corsMiddleware(statsHandler))
	http.HandleFunc("/jobs/recent", corsMiddleware(recentJobsHandler))

	port := getEnv("PORT", "8080")
	log.Printf("Listening on :%s", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}
