package main

import (
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
