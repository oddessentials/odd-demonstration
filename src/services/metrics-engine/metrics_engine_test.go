package main

import (
	"encoding/json"
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

// ============================================================
// Tests for struct serialization (EventEnvelope, DLQMessage)
// ============================================================

// TestEventEnvelopeJSONSerialization tests EventEnvelope JSON roundtrip.
func TestEventEnvelopeJSONSerialization(t *testing.T) {
	envelope := EventEnvelope{
		ContractVersion: "1.0.0",
		EventType:       "job.created",
		EventID:         "evt-123",
		OccurredAt:      "2024-01-01T00:00:00Z",
		CorrelationID:   "corr-456",
		Payload:         map[string]interface{}{"id": "job-789", "type": "compute"},
	}

	// Marshal to JSON
	data, err := json.Marshal(envelope)
	if err != nil {
		t.Fatalf("Failed to marshal EventEnvelope: %v", err)
	}

	// Unmarshal back
	var decoded EventEnvelope
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal EventEnvelope: %v", err)
	}

	// Verify fields
	if decoded.ContractVersion != "1.0.0" {
		t.Errorf("Expected contractVersion '1.0.0', got '%s'", decoded.ContractVersion)
	}
	if decoded.EventType != "job.created" {
		t.Errorf("Expected eventType 'job.created', got '%s'", decoded.EventType)
	}
	if decoded.EventID != "evt-123" {
		t.Errorf("Expected eventId 'evt-123', got '%s'", decoded.EventID)
	}
	if decoded.CorrelationID != "corr-456" {
		t.Errorf("Expected correlationId 'corr-456', got '%s'", decoded.CorrelationID)
	}
}

// TestEventEnvelopeJSONFieldNames tests that JSON field names are correct.
func TestEventEnvelopeJSONFieldNames(t *testing.T) {
	envelope := EventEnvelope{
		ContractVersion: "1.0.0",
		EventType:       "job.failed",
		EventID:         "evt-001",
		OccurredAt:      "2024-06-15T12:00:00Z",
		CorrelationID:   "corr-001",
		Payload:         nil,
	}

	data, _ := json.Marshal(envelope)
	jsonStr := string(data)

	// Verify camelCase field names in JSON output
	expectedFields := []string{"contractVersion", "eventType", "eventId", "occurredAt", "correlationId", "payload"}
	for _, field := range expectedFields {
		if !strings.Contains(jsonStr, field) {
			t.Errorf("Expected JSON to contain field '%s', got: %s", field, jsonStr)
		}
	}
}

// TestDLQMessageJSONSerialization tests DLQMessage JSON roundtrip.
func TestDLQMessageJSONSerialization(t *testing.T) {
	dlqMsg := DLQMessage{
		OriginalEvent:   json.RawMessage(`{"eventType":"job.failed"}`),
		ValidationError: "missing required field: eventId",
		RejectedAt:      "2024-01-01T00:00:00Z",
		CorrelationID:   "corr-789",
		Service:         "metrics-engine",
	}

	// Marshal to JSON
	data, err := json.Marshal(dlqMsg)
	if err != nil {
		t.Fatalf("Failed to marshal DLQMessage: %v", err)
	}

	// Unmarshal back
	var decoded DLQMessage
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal DLQMessage: %v", err)
	}

	// Verify fields
	if decoded.ValidationError != "missing required field: eventId" {
		t.Errorf("Expected validation_error, got '%s'", decoded.ValidationError)
	}
	if decoded.Service != "metrics-engine" {
		t.Errorf("Expected service 'metrics-engine', got '%s'", decoded.Service)
	}
	if decoded.CorrelationID != "corr-789" {
		t.Errorf("Expected correlation_id 'corr-789', got '%s'", decoded.CorrelationID)
	}
}

// TestDLQMessageJSONFieldNames tests that DLQ JSON field names use snake_case.
func TestDLQMessageJSONFieldNames(t *testing.T) {
	dlqMsg := DLQMessage{
		OriginalEvent:   json.RawMessage(`{}`),
		ValidationError: "error",
		RejectedAt:      "2024-01-01T00:00:00Z",
		CorrelationID:   "corr-001",
		Service:         "test",
	}

	data, _ := json.Marshal(dlqMsg)
	jsonStr := string(data)

	// Verify snake_case field names in JSON output
	expectedFields := []string{"original_event", "validation_error", "rejected_at", "correlation_id", "service"}
	for _, field := range expectedFields {
		if !strings.Contains(jsonStr, field) {
			t.Errorf("Expected JSON to contain field '%s', got: %s", field, jsonStr)
		}
	}
}

// TestDLQNameConstant tests the DLQ queue name constant.
func TestDLQNameConstant(t *testing.T) {
	if DLQName != "jobs.failed.validation" {
		t.Errorf("Expected DLQName 'jobs.failed.validation', got '%s'", DLQName)
	}
}

// ============================================================
// Additional edge case tests
// ============================================================

// TestFormatValidationErrorsThreeErrors tests formatValidationErrors with three errors.
func TestFormatValidationErrorsThreeErrors(t *testing.T) {
	errors := []validator.ValidationError{
		{Field: "field1", Message: "error1"},
		{Field: "field2", Message: "error2"},
		{Field: "field3", Message: "error3"},
	}
	result := formatValidationErrors(errors)
	expected := "field1: error1; field2: error2; field3: error3"
	if result != expected {
		t.Errorf("Expected '%s', got '%s'", expected, result)
	}
}

// TestFormatValidationErrorsSpecialChars tests with special characters in messages.
func TestFormatValidationErrorsSpecialChars(t *testing.T) {
	errors := []validator.ValidationError{
		{Field: "$.payload.id", Message: "must be a valid UUID (got: 'invalid')"},
	}
	result := formatValidationErrors(errors)
	if !strings.Contains(result, "$.payload.id") {
		t.Errorf("Expected field path in result, got '%s'", result)
	}
	if !strings.Contains(result, "UUID") {
		t.Errorf("Expected error message in result, got '%s'", result)
	}
}

// TestGetEnvMultipleCalls tests getEnv with multiple different keys.
func TestGetEnvMultipleCalls(t *testing.T) {
	os.Setenv("TEST_KEY_A", "value_a")
	os.Setenv("TEST_KEY_B", "value_b")
	defer os.Unsetenv("TEST_KEY_A")
	defer os.Unsetenv("TEST_KEY_B")

	resultA := getEnv("TEST_KEY_A", "default")
	resultB := getEnv("TEST_KEY_B", "default")
	resultC := getEnv("TEST_KEY_C", "default_c")

	if resultA != "value_a" {
		t.Errorf("Expected 'value_a', got '%s'", resultA)
	}
	if resultB != "value_b" {
		t.Errorf("Expected 'value_b', got '%s'", resultB)
	}
	if resultC != "default_c" {
		t.Errorf("Expected 'default_c', got '%s'", resultC)
	}
}

// TestEventEnvelopeWithNilPayload tests EventEnvelope with nil payload.
func TestEventEnvelopeWithNilPayload(t *testing.T) {
	envelope := EventEnvelope{
		ContractVersion: "1.0.0",
		EventType:       "system.heartbeat",
		EventID:         "evt-hb",
		OccurredAt:      "2024-01-01T00:00:00Z",
		CorrelationID:   "corr-hb",
		Payload:         nil,
	}

	data, err := json.Marshal(envelope)
	if err != nil {
		t.Fatalf("Failed to marshal EventEnvelope with nil payload: %v", err)
	}

	// Should still produce valid JSON
	if len(data) == 0 {
		t.Error("Expected non-empty JSON output")
	}
}

// TestEventEnvelopeWithComplexPayload tests EventEnvelope with nested payload.
func TestEventEnvelopeWithComplexPayload(t *testing.T) {
	envelope := EventEnvelope{
		ContractVersion: "1.0.0",
		EventType:       "job.completed",
		EventID:         "evt-complex",
		OccurredAt:      "2024-01-01T00:00:00Z",
		CorrelationID:   "corr-complex",
		Payload: map[string]interface{}{
			"id":     "job-123",
			"type":   "compute",
			"status": "COMPLETED",
			"metadata": map[string]interface{}{
				"duration_ms": 1500,
				"worker_id":   "worker-1",
			},
		},
	}

	data, err := json.Marshal(envelope)
	if err != nil {
		t.Fatalf("Failed to marshal EventEnvelope with complex payload: %v", err)
	}

	jsonStr := string(data)
	if !strings.Contains(jsonStr, "metadata") {
		t.Error("Expected nested metadata in JSON output")
	}
	if !strings.Contains(jsonStr, "duration_ms") {
		t.Error("Expected duration_ms in JSON output")
	}
}
