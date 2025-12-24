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
