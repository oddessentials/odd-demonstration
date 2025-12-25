package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// ReadVersion reads the VERSION file and returns the version string.
func ReadVersion() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", err
	}

	versionPath := filepath.Join(dir, "VERSION")
	data, err := os.ReadFile(versionPath)
	if err != nil {
		return "", err
	}

	version := strings.TrimSpace(string(data))
	return version, nil
}

// IsValidSemVer checks if a version string matches SemVer format.
func IsValidSemVer(version string) bool {
	pattern := regexp.MustCompile(`^\d+\.\d+\.\d+$`)
	return pattern.MatchString(version)
}

// buildTestHealthResponse creates a health response for testing.
func buildTestHealthResponse(version string) HealthResponse {
	return HealthResponse{
		Status:  "ok",
		Version: version,
	}
}

// testMetricLabels creates metric labels for testing.
func testMetricLabels(service, version string) map[string]string {
	return map[string]string{
		"service": service,
		"version": version,
	}
}

// TestReadVersion tests that VERSION file returns valid SemVer.
func TestReadVersion(t *testing.T) {
	tmpDir := t.TempDir()
	versionPath := filepath.Join(tmpDir, "VERSION")
	err := os.WriteFile(versionPath, []byte("0.1.0\n"), 0644)
	if err != nil {
		t.Fatalf("Failed to create VERSION file: %v", err)
	}

	oldDir, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldDir)

	version, err := ReadVersion()
	if err != nil {
		t.Fatalf("ReadVersion failed: %v", err)
	}

	if !IsValidSemVer(version) {
		t.Errorf("Invalid SemVer format: %s", version)
	}

	if version != "0.1.0" {
		t.Errorf("Expected 0.1.0, got %s", version)
	}
}

// TestHealthResponseIncludesVersion tests health response contains version.
func TestHealthResponseIncludesVersion(t *testing.T) {
	version := "0.1.0"
	response := buildTestHealthResponse(version)

	if response.Status != "ok" {
		t.Errorf("Expected status 'ok', got '%s'", response.Status)
	}

	if response.Version != version {
		t.Errorf("Expected version '%s', got '%s'", version, response.Version)
	}
}

// TestMetricLabelsIncludeVersion tests metric labels contain version.
func TestMetricLabelsIncludeVersion(t *testing.T) {
	labels := testMetricLabels("read-model", "0.1.0")

	if labels["service"] != "read-model" {
		t.Errorf("Expected service 'read-model', got '%s'", labels["service"])
	}

	if labels["version"] != "0.1.0" {
		t.Errorf("Expected version '0.1.0', got '%s'", labels["version"])
	}
}

// ============================================================
// Tests for main.go functions
// ============================================================

// TestGetEnvWithValue tests getEnv returns env var when set.
func TestGetEnvWithValue(t *testing.T) {
	os.Setenv("TEST_READ_MODEL_VAR", "custom_value")
	defer os.Unsetenv("TEST_READ_MODEL_VAR")

	result := getEnv("TEST_READ_MODEL_VAR", "default")
	if result != "custom_value" {
		t.Errorf("Expected 'custom_value', got '%s'", result)
	}
}

// TestGetEnvWithFallback tests getEnv returns fallback when not set.
func TestGetEnvWithFallback(t *testing.T) {
	os.Unsetenv("NONEXISTENT_VAR")

	result := getEnv("NONEXISTENT_VAR", "fallback_value")
	if result != "fallback_value" {
		t.Errorf("Expected 'fallback_value', got '%s'", result)
	}
}

// TestGetEnvEmptyValue tests getEnv with empty string value.
func TestGetEnvEmptyValue(t *testing.T) {
	os.Setenv("TEST_EMPTY_VAR", "")
	defer os.Unsetenv("TEST_EMPTY_VAR")

	result := getEnv("TEST_EMPTY_VAR", "default")
	if result != "" {
		t.Errorf("Expected empty string, got '%s'", result)
	}
}

// TestStatsResponseStruct tests StatsResponse struct creation.
func TestStatsResponseStruct(t *testing.T) {
	stats := StatsResponse{
		TotalJobs:     100,
		CompletedJobs: 80,
		FailedJobs:    5,
		LastEventTime: "2024-01-01T00:00:00Z",
	}

	if stats.TotalJobs != 100 {
		t.Errorf("Expected TotalJobs 100, got %d", stats.TotalJobs)
	}
	if stats.CompletedJobs != 80 {
		t.Errorf("Expected CompletedJobs 80, got %d", stats.CompletedJobs)
	}
	if stats.FailedJobs != 5 {
		t.Errorf("Expected FailedJobs 5, got %d", stats.FailedJobs)
	}
}

// TestJobStruct tests Job struct creation.
func TestJobStruct(t *testing.T) {
	job := Job{
		ID:        "job-123",
		Type:      "compute",
		Status:    "PENDING",
		CreatedAt: "2024-01-01T00:00:00Z",
	}

	if job.ID != "job-123" {
		t.Errorf("Expected ID 'job-123', got '%s'", job.ID)
	}
	if job.Status != "PENDING" {
		t.Errorf("Expected Status 'PENDING', got '%s'", job.Status)
	}
}

// ============================================================
// Tests for OpenAPI documentation endpoints
// ============================================================

// TestOpenApiHandler tests the /openapi.json endpoint returns valid JSON.
func TestOpenApiHandler(t *testing.T) {
	// Set ServiceVersion for test
	ServiceVersion = "0.1.0"

	req := httptest.NewRequest("GET", "/openapi.json", nil)
	w := httptest.NewRecorder()

	openApiHandler(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status 200, got %d", resp.StatusCode)
	}

	contentType := resp.Header.Get("Content-Type")
	if contentType != "application/json" {
		t.Errorf("Expected Content-Type 'application/json', got '%s'", contentType)
	}

	// Parse response as JSON
	var spec map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&spec); err != nil {
		t.Fatalf("Failed to decode OpenAPI spec: %v", err)
	}

	// Verify OpenAPI structure
	if spec["openapi"] != "3.0.3" {
		t.Errorf("Expected openapi '3.0.3', got '%v'", spec["openapi"])
	}

	info, ok := spec["info"].(map[string]interface{})
	if !ok {
		t.Fatal("Expected 'info' object in spec")
	}

	if info["title"] != "Read Model API" {
		t.Errorf("Expected title 'Read Model API', got '%v'", info["title"])
	}

	if info["version"] != "0.1.0" {
		t.Errorf("Expected version '0.1.0', got '%v'", info["version"])
	}
}

// TestOpenApiHandlerIncludesAllPaths tests that all API paths are documented.
func TestOpenApiHandlerIncludesAllPaths(t *testing.T) {
	ServiceVersion = "0.1.0"

	req := httptest.NewRequest("GET", "/openapi.json", nil)
	w := httptest.NewRecorder()

	openApiHandler(w, req)

	var spec map[string]interface{}
	json.NewDecoder(w.Result().Body).Decode(&spec)

	paths, ok := spec["paths"].(map[string]interface{})
	if !ok {
		t.Fatal("Expected 'paths' object in spec")
	}

	requiredPaths := []string{"/health", "/stats", "/jobs/recent", "/events"}
	for _, path := range requiredPaths {
		if _, exists := paths[path]; !exists {
			t.Errorf("Expected path '%s' to be documented", path)
		}
	}
}

// TestDocsHandler tests the /docs endpoint returns HTML.
func TestDocsHandler(t *testing.T) {
	req := httptest.NewRequest("GET", "/docs", nil)
	w := httptest.NewRecorder()

	docsHandler(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status 200, got %d", resp.StatusCode)
	}

	contentType := resp.Header.Get("Content-Type")
	if contentType != "text/html" {
		t.Errorf("Expected Content-Type 'text/html', got '%s'", contentType)
	}

	// Read body and verify it contains Swagger UI elements
	body := w.Body.String()
	if !strings.Contains(body, "swagger-ui") {
		t.Error("Expected HTML to contain 'swagger-ui'")
	}
	if !strings.Contains(body, "/openapi.json") {
		t.Error("Expected HTML to reference '/openapi.json'")
	}
}

// TestOpenApiContactInfo tests that contact info is included.
func TestOpenApiContactInfo(t *testing.T) {
	ServiceVersion = "0.1.0"

	req := httptest.NewRequest("GET", "/openapi.json", nil)
	w := httptest.NewRecorder()

	openApiHandler(w, req)

	var spec map[string]interface{}
	json.NewDecoder(w.Result().Body).Decode(&spec)

	info := spec["info"].(map[string]interface{})
	contact, ok := info["contact"].(map[string]interface{})
	if !ok {
		t.Fatal("Expected 'contact' object in info")
	}

	if contact["name"] != "Odd Essentials" {
		t.Errorf("Expected contact name 'Odd Essentials', got '%v'", contact["name"])
	}

	if contact["url"] != "https://oddessentials.com" {
		t.Errorf("Expected contact url 'https://oddessentials.com', got '%v'", contact["url"])
	}
}

// ============================================================
// Tests for CORS middleware
// ============================================================

// TestCorsMiddlewareAddsHeaders tests CORS headers are added.
func TestCorsMiddlewareAddsHeaders(t *testing.T) {
	handler := corsMiddleware(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	req := httptest.NewRequest("GET", "/test", nil)
	w := httptest.NewRecorder()

	handler(w, req)

	resp := w.Result()
	if resp.Header.Get("Access-Control-Allow-Origin") != "*" {
		t.Errorf("Expected Access-Control-Allow-Origin '*', got '%s'", resp.Header.Get("Access-Control-Allow-Origin"))
	}
	if resp.Header.Get("Access-Control-Allow-Methods") != "GET, OPTIONS" {
		t.Errorf("Expected Access-Control-Allow-Methods 'GET, OPTIONS', got '%s'", resp.Header.Get("Access-Control-Allow-Methods"))
	}
	if resp.Header.Get("Access-Control-Allow-Headers") != "Content-Type" {
		t.Errorf("Expected Access-Control-Allow-Headers 'Content-Type', got '%s'", resp.Header.Get("Access-Control-Allow-Headers"))
	}
}

// TestCorsMiddlewareOptionsRequest tests OPTIONS preflight handling.
func TestCorsMiddlewareOptionsRequest(t *testing.T) {
	handlerCalled := false
	handler := corsMiddleware(func(w http.ResponseWriter, r *http.Request) {
		handlerCalled = true
		w.WriteHeader(http.StatusOK)
	})

	req := httptest.NewRequest("OPTIONS", "/test", nil)
	w := httptest.NewRecorder()

	handler(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status 200, got %d", resp.StatusCode)
	}
	if handlerCalled {
		t.Error("Handler should not be called for OPTIONS request")
	}
}

// TestCorsMiddlewarePassesThrough tests that non-OPTIONS requests pass through.
func TestCorsMiddlewarePassesThrough(t *testing.T) {
	handlerCalled := false
	handler := corsMiddleware(func(w http.ResponseWriter, r *http.Request) {
		handlerCalled = true
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("success"))
	})

	req := httptest.NewRequest("GET", "/test", nil)
	w := httptest.NewRecorder()

	handler(w, req)

	if !handlerCalled {
		t.Error("Handler should be called for GET request")
	}
	if w.Body.String() != "success" {
		t.Errorf("Expected body 'success', got '%s'", w.Body.String())
	}
}

// ============================================================
// Tests for health handler
// ============================================================

// TestHealthHandler tests the health endpoint.
func TestHealthHandler(t *testing.T) {
	ServiceVersion = "1.2.3"

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	healthHandler(w, req)

	resp := w.Result()
	if resp.StatusCode != http.StatusOK {
		t.Errorf("Expected status 200, got %d", resp.StatusCode)
	}

	contentType := resp.Header.Get("Content-Type")
	if contentType != "application/json" {
		t.Errorf("Expected Content-Type 'application/json', got '%s'", contentType)
	}

	var health HealthResponse
	if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
		t.Fatalf("Failed to decode health response: %v", err)
	}

	if health.Status != "ok" {
		t.Errorf("Expected status 'ok', got '%s'", health.Status)
	}
	if health.Version != "1.2.3" {
		t.Errorf("Expected version '1.2.3', got '%s'", health.Version)
	}
}

// ============================================================
// Additional struct serialization tests
// ============================================================

// TestStatsResponseJSONSerialization tests StatsResponse JSON roundtrip.
func TestStatsResponseJSONSerialization(t *testing.T) {
	stats := StatsResponse{
		TotalJobs:     1000,
		CompletedJobs: 850,
		FailedJobs:    50,
		LastEventTime: "2024-12-25T12:00:00Z",
	}

	data, err := json.Marshal(stats)
	if err != nil {
		t.Fatalf("Failed to marshal StatsResponse: %v", err)
	}

	var decoded StatsResponse
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal StatsResponse: %v", err)
	}

	if decoded.TotalJobs != 1000 {
		t.Errorf("Expected TotalJobs 1000, got %d", decoded.TotalJobs)
	}
	if decoded.CompletedJobs != 850 {
		t.Errorf("Expected CompletedJobs 850, got %d", decoded.CompletedJobs)
	}
	if decoded.FailedJobs != 50 {
		t.Errorf("Expected FailedJobs 50, got %d", decoded.FailedJobs)
	}
	if decoded.LastEventTime != "2024-12-25T12:00:00Z" {
		t.Errorf("Expected LastEventTime '2024-12-25T12:00:00Z', got '%s'", decoded.LastEventTime)
	}
}

// TestStatsResponseJSONFieldNames tests JSON field names are camelCase.
func TestStatsResponseJSONFieldNames(t *testing.T) {
	stats := StatsResponse{
		TotalJobs:     100,
		CompletedJobs: 80,
		FailedJobs:    5,
		LastEventTime: "2024-01-01T00:00:00Z",
	}

	data, _ := json.Marshal(stats)
	jsonStr := string(data)

	expectedFields := []string{"totalJobs", "completedJobs", "failedJobs", "lastEventTime"}
	for _, field := range expectedFields {
		if !strings.Contains(jsonStr, field) {
			t.Errorf("Expected JSON to contain field '%s', got: %s", field, jsonStr)
		}
	}
}

// TestJobJSONSerialization tests Job struct JSON roundtrip.
func TestJobJSONSerialization(t *testing.T) {
	job := Job{
		ID:        "job-abc-123",
		Type:      "data-processing",
		Status:    "COMPLETED",
		CreatedAt: "2024-12-25T10:00:00Z",
	}

	data, err := json.Marshal(job)
	if err != nil {
		t.Fatalf("Failed to marshal Job: %v", err)
	}

	var decoded Job
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal Job: %v", err)
	}

	if decoded.ID != "job-abc-123" {
		t.Errorf("Expected ID 'job-abc-123', got '%s'", decoded.ID)
	}
	if decoded.Type != "data-processing" {
		t.Errorf("Expected Type 'data-processing', got '%s'", decoded.Type)
	}
	if decoded.Status != "COMPLETED" {
		t.Errorf("Expected Status 'COMPLETED', got '%s'", decoded.Status)
	}
}

// TestHealthResponseJSONFieldNames tests HealthResponse JSON field names.
func TestHealthResponseJSONFieldNames(t *testing.T) {
	health := HealthResponse{
		Status:  "ok",
		Version: "1.0.0",
	}

	data, _ := json.Marshal(health)
	jsonStr := string(data)

	if !strings.Contains(jsonStr, "status") {
		t.Errorf("Expected JSON to contain 'status', got: %s", jsonStr)
	}
	if !strings.Contains(jsonStr, "version") {
		t.Errorf("Expected JSON to contain 'version', got: %s", jsonStr)
	}
}

// ============================================================
// Edge case tests
// ============================================================

// TestGetEnvMultipleCalls tests getEnv with multiple keys.
func TestGetEnvMultipleCalls(t *testing.T) {
	os.Setenv("RM_TEST_A", "alpha")
	os.Setenv("RM_TEST_B", "beta")
	defer os.Unsetenv("RM_TEST_A")
	defer os.Unsetenv("RM_TEST_B")

	if getEnv("RM_TEST_A", "x") != "alpha" {
		t.Error("Expected 'alpha' for RM_TEST_A")
	}
	if getEnv("RM_TEST_B", "x") != "beta" {
		t.Error("Expected 'beta' for RM_TEST_B")
	}
	if getEnv("RM_TEST_C", "gamma") != "gamma" {
		t.Error("Expected fallback 'gamma' for RM_TEST_C")
	}
}

// TestJobStatusValues tests Job with different status values.
func TestJobStatusValues(t *testing.T) {
	statuses := []string{"PENDING", "RUNNING", "COMPLETED", "FAILED", "CANCELLED"}
	for _, status := range statuses {
		job := Job{
			ID:        "job-1",
			Type:      "test",
			Status:    status,
			CreatedAt: "2024-01-01T00:00:00Z",
		}
		data, err := json.Marshal(job)
		if err != nil {
			t.Errorf("Failed to marshal Job with status '%s': %v", status, err)
		}
		if !strings.Contains(string(data), status) {
			t.Errorf("Expected status '%s' in JSON output", status)
		}
	}
}

// TestStatsResponseZeroValues tests StatsResponse with zero values.
func TestStatsResponseZeroValues(t *testing.T) {
	stats := StatsResponse{
		TotalJobs:     0,
		CompletedJobs: 0,
		FailedJobs:    0,
		LastEventTime: "",
	}

	data, err := json.Marshal(stats)
	if err != nil {
		t.Fatalf("Failed to marshal StatsResponse with zeros: %v", err)
	}

	var decoded StatsResponse
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal StatsResponse: %v", err)
	}

	if decoded.TotalJobs != 0 {
		t.Errorf("Expected TotalJobs 0, got %d", decoded.TotalJobs)
	}
}

// TestOpenApiVersionDynamic tests that OpenAPI version comes from ServiceVersion.
func TestOpenApiVersionDynamic(t *testing.T) {
	ServiceVersion = "99.88.77"

	req := httptest.NewRequest("GET", "/openapi.json", nil)
	w := httptest.NewRecorder()

	openApiHandler(w, req)

	var spec map[string]interface{}
	json.NewDecoder(w.Result().Body).Decode(&spec)

	info := spec["info"].(map[string]interface{})
	if info["version"] != "99.88.77" {
		t.Errorf("Expected version '99.88.77', got '%v'", info["version"])
	}
}

// TestDocsHandlerContainsSwaggerUI tests docs endpoint has SwaggerUI.
func TestDocsHandlerContainsSwaggerUI(t *testing.T) {
	req := httptest.NewRequest("GET", "/docs", nil)
	w := httptest.NewRecorder()

	docsHandler(w, req)

	body := w.Body.String()
	requiredElements := []string{
		"<!DOCTYPE html>",
		"swagger-ui",
		"SwaggerUIBundle",
		"/openapi.json",
		"dom_id",
	}
	for _, element := range requiredElements {
		if !strings.Contains(body, element) {
			t.Errorf("Expected docs HTML to contain '%s'", element)
		}
	}
}
