package main

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"

	"github.com/distributed-task-observatory/metrics-engine/validator"
)

// ReadVersion reads the VERSION file and returns the version string.
func ReadVersion() (string, error) {
	// Get the directory of the executable or test
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

// HealthResponse represents the health endpoint response.
type HealthResponse struct {
	Status  string `json:"status"`
	Version string `json:"version"`
}

// BuildHealthResponse creates a health response with version.
func BuildHealthResponse(version string) HealthResponse {
	return HealthResponse{
		Status:  "ok",
		Version: version,
	}
}

// MetricLabels includes version in metric labels.
func MetricLabels(service, version string) map[string]string {
	return map[string]string{
		"service": service,
		"version": version,
	}
}

// TestReadVersion tests that VERSION file returns valid SemVer.
func TestReadVersion(t *testing.T) {
	// Create a temporary VERSION file for testing
	tmpDir := t.TempDir()
	versionPath := filepath.Join(tmpDir, "VERSION")
	err := os.WriteFile(versionPath, []byte("0.1.0\n"), 0644)
	if err != nil {
		t.Fatalf("Failed to create VERSION file: %v", err)
	}

	// Change to temp dir and restore after
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
	response := BuildHealthResponse(version)

	if response.Status != "ok" {
		t.Errorf("Expected status 'ok', got '%s'", response.Status)
	}

	if response.Version != version {
		t.Errorf("Expected version '%s', got '%s'", version, response.Version)
	}

	if response.Version == "" {
		t.Error("Version should not be empty")
	}
}

// TestMetricLabelsIncludeVersion tests metric labels contain version.
func TestMetricLabelsIncludeVersion(t *testing.T) {
	labels := MetricLabels("metrics-engine", "0.1.0")

	if labels["service"] != "metrics-engine" {
		t.Errorf("Expected service 'metrics-engine', got '%s'", labels["service"])
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
	os.Setenv("TEST_METRICS_VAR", "custom_value")
	defer os.Unsetenv("TEST_METRICS_VAR")

	result := getEnv("TEST_METRICS_VAR", "default")
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
	// Empty string is still a set value
	if result != "" {
		t.Errorf("Expected empty string, got '%s'", result)
	}
}

// TestFormatValidationErrorsEmpty tests formatValidationErrors with no errors.
func TestFormatValidationErrorsEmpty(t *testing.T) {
	result := formatValidationErrors(nil)
	if result != "unknown error" {
		t.Errorf("Expected 'unknown error', got '%s'", result)
	}

	result2 := formatValidationErrors([]validator.ValidationError{})
	if result2 != "unknown error" {
		t.Errorf("Expected 'unknown error', got '%s'", result2)
	}
}

// TestFormatValidationErrorsSingle tests formatValidationErrors with one error.
func TestFormatValidationErrorsSingle(t *testing.T) {
	errors := []validator.ValidationError{
		{Field: "eventId", Message: "required field missing"},
	}
	result := formatValidationErrors(errors)
	expected := "eventId: required field missing"
	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}

// TestFormatValidationErrorsMultiple tests formatValidationErrors with multiple errors.
func TestFormatValidationErrorsMultiple(t *testing.T) {
	errors := []validator.ValidationError{
		{Field: "eventId", Message: "required"},
		{Field: "producer", Message: "invalid format"},
	}
	result := formatValidationErrors(errors)
	expected := "eventId: required; producer: invalid format"
	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}
