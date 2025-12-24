package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"regexp"
	"strings"
	"time"

	_ "github.com/lib/pq"
	"github.com/redis/go-redis/v9"
	"go.mongodb.org/mongo-driver/bson"
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

type StatsResponse struct {
	TotalJobs     int64  `json:"totalJobs"`
	CompletedJobs int64  `json:"completedJobs"`
	FailedJobs    int64  `json:"failedJobs"`
	LastEventTime string `json:"lastEventTime"`
}

type HealthResponse struct {
	Status  string `json:"status"`
	Version string `json:"version"`
}

type Job struct {
	ID        string `json:"id"`
	Type      string `json:"type"`
	Status    string `json:"status"`
	CreatedAt string `json:"createdAt"`
}

var rdb *redis.Client
var db *sql.DB
var mongoClient *mongo.Client
var eventsColl *mongo.Collection
var ctx = context.Background()

func getEnv(key, fallback string) string {
	if value, ok := os.LookupEnv(key); ok {
		return value
	}
	return fallback
}

func healthHandler(w http.ResponseWriter, r *http.Request) {
	resp := HealthResponse{
		Status:  "ok",
		Version: ServiceVersion,
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(resp)
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

func eventsHandler(w http.ResponseWriter, r *http.Request) {
	jobID := r.URL.Query().Get("jobId")
	filter := bson.M{}
	if jobID != "" {
		filter = bson.M{"payload.id": jobID}
	}

	opts := options.Find().SetLimit(50).SetSort(bson.M{"occurredAt": -1})
	cursor, err := eventsColl.Find(ctx, filter, opts)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	defer cursor.Close(ctx)

	var events []interface{}
	if err = cursor.All(ctx, &events); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(events)
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

// OpenAPI specification handler
func openApiHandler(w http.ResponseWriter, r *http.Request) {
	openApiSpec := map[string]interface{}{
		"openapi": "3.0.3",
		"info": map[string]interface{}{
			"title":       "Read Model API",
			"description": "Distributed Task Observatory Read Model Service - provides aggregated statistics and query endpoints",
			"version":     ServiceVersion,
			"contact": map[string]string{
				"name": "Odd Essentials",
				"url":  "https://oddessentials.com",
			},
		},
		"servers": []map[string]string{
			{"url": "http://localhost:8080", "description": "Local development"},
		},
		"paths": map[string]interface{}{
			"/health": map[string]interface{}{
				"get": map[string]interface{}{
					"summary": "Health check",
					"responses": map[string]interface{}{
						"200": map[string]string{"description": "Service healthy"},
					},
				},
			},
			"/stats": map[string]interface{}{
				"get": map[string]interface{}{
					"summary":     "Get job statistics",
					"description": "Returns aggregated job statistics from Redis cache",
					"responses": map[string]interface{}{
						"200": map[string]string{"description": "Statistics object with totalJobs, completedJobs, failedJobs, lastEventTime"},
					},
				},
			},
			"/jobs/recent": map[string]interface{}{
				"get": map[string]interface{}{
					"summary":     "Get recent jobs",
					"description": "Returns the 10 most recent jobs from PostgreSQL",
					"responses": map[string]interface{}{
						"200": map[string]string{"description": "Array of job objects"},
					},
				},
			},
			"/events": map[string]interface{}{
				"get": map[string]interface{}{
					"summary":     "Get job events",
					"description": "Returns events from MongoDB, optionally filtered by jobId query parameter",
					"parameters": []map[string]interface{}{
						{"name": "jobId", "in": "query", "required": false, "schema": map[string]string{"type": "string"}},
					},
					"responses": map[string]interface{}{
						"200": map[string]string{"description": "Array of event objects"},
					},
				},
			},
		},
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(openApiSpec)
}

// HTML documentation page handler
func docsHandler(w http.ResponseWriter, r *http.Request) {
	htmlContent := `<!DOCTYPE html>
<html>
<head>
    <title>Read Model API Documentation</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        SwaggerUIBundle({
            url: '/openapi.json',
            dom_id: '#swagger-ui',
            presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
            layout: 'BaseLayout'
        });
    </script>
</body>
</html>`
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(htmlContent))
}

func main() {
	// Read and validate version at startup
	ServiceVersion = readVersion()
	log.Printf("Read Model API version %s starting...", ServiceVersion)

	redisURL := getEnv("REDIS_URL", "redis:6379")
	postgresURL := getEnv("POSTGRES_URL", "postgres://admin:password123@postgres:5432/task_db?sslmode=disable")
	mongoURL := getEnv("MONGO_URL", "mongodb://admin:password123@mongodb:27017")

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

	// Connect to MongoDB
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
	eventsColl = mongoClient.Database("observatory").Collection("job_events")

	http.HandleFunc("/health", corsMiddleware(healthHandler))
	http.HandleFunc("/stats", corsMiddleware(statsHandler))
	http.HandleFunc("/jobs/recent", corsMiddleware(recentJobsHandler))
	http.HandleFunc("/events", corsMiddleware(eventsHandler))
	http.HandleFunc("/openapi.json", corsMiddleware(openApiHandler))
	http.HandleFunc("/docs", corsMiddleware(docsHandler))

	port := getEnv("PORT", "8080")
	log.Printf("Listening on :%s", port)
	log.Fatal(http.ListenAndServe(":"+port, nil))
}
