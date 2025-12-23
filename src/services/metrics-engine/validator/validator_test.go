package validator

import (
	"testing"
)

// TestNewValidator tests that NewValidator successfully loads schemas.
func TestNewValidator(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}
	if v == nil {
		t.Fatal("NewValidator returned nil")
	}
	if len(v.schemas) != 2 {
		t.Errorf("Expected 2 schemas loaded, got %d", len(v.schemas))
	}
}

// TestGetCorrelationIDValid tests extraction of correlation ID from valid message.
func TestGetCorrelationIDValid(t *testing.T) {
	message := []byte(`{"correlationId": "abc-123-def"}`)
	result := GetCorrelationID(message)
	if result != "abc-123-def" {
		t.Errorf("Expected 'abc-123-def', got '%s'", result)
	}
}

// TestGetCorrelationIDEmpty tests GetCorrelationID with empty correlation ID.
func TestGetCorrelationIDEmpty(t *testing.T) {
	message := []byte(`{"correlationId": ""}`)
	result := GetCorrelationID(message)
	if result != "unknown" {
		t.Errorf("Expected 'unknown' for empty correlationId, got '%s'", result)
	}
}

// TestGetCorrelationIDMissing tests GetCorrelationID with missing field.
func TestGetCorrelationIDMissing(t *testing.T) {
	message := []byte(`{"eventType": "job.created"}`)
	result := GetCorrelationID(message)
	if result != "unknown" {
		t.Errorf("Expected 'unknown' for missing correlationId, got '%s'", result)
	}
}

// TestGetCorrelationIDInvalidJSON tests GetCorrelationID with invalid JSON.
func TestGetCorrelationIDInvalidJSON(t *testing.T) {
	message := []byte(`not valid json`)
	result := GetCorrelationID(message)
	if result != "unknown" {
		t.Errorf("Expected 'unknown' for invalid JSON, got '%s'", result)
	}
}

// TestValidateEventEnvelopeValid tests validation of a valid event envelope.
func TestValidateEventEnvelopeValid(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}

	validEnvelope := []byte(`{
		"contractVersion": "1.0.0",
		"eventType": "job.created",
		"eventId": "550e8400-e29b-41d4-a716-446655440000",
		"occurredAt": "2024-01-01T00:00:00Z",
		"correlationId": "550e8400-e29b-41d4-a716-446655440001",
		"idempotencyKey": "550e8400-e29b-41d4-a716-446655440002",
		"producer": {
			"service": "test-service",
			"instanceId": "instance-1",
			"version": "0.1.0"
		},
		"payload": {"id": "550e8400-e29b-41d4-a716-446655440003", "type": "test", "status": "PENDING", "createdAt": "2024-01-01T00:00:00Z"}
	}`)

	result := v.ValidateEventEnvelope(validEnvelope)
	if !result.Valid {
		t.Errorf("Expected valid, got invalid with errors: %+v", result.Errors)
	}
}

// TestValidateEventEnvelopeInvalid tests validation of an invalid envelope.
func TestValidateEventEnvelopeInvalid(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}

	// Missing required fields
	invalidEnvelope := []byte(`{"eventType": "job.created"}`)

	result := v.ValidateEventEnvelope(invalidEnvelope)
	if result.Valid {
		t.Error("Expected invalid, got valid")
	}
	if len(result.Errors) == 0 {
		t.Error("Expected errors for missing required fields")
	}
}

// TestValidateJobValid tests validation of a valid job payload.
func TestValidateJobValid(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}

	validJob := []byte(`{
		"id": "550e8400-e29b-41d4-a716-446655440000",
		"type": "compute",
		"status": "PENDING",
		"createdAt": "2024-01-01T00:00:00Z"
	}`)

	result := v.ValidateJob(validJob)
	if !result.Valid {
		t.Errorf("Expected valid, got invalid with errors: %+v", result.Errors)
	}
}

// TestValidateJobInvalidStatus tests validation with invalid status enum.
func TestValidateJobInvalidStatus(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}

	invalidJob := []byte(`{
		"id": "550e8400-e29b-41d4-a716-446655440000",
		"type": "compute",
		"status": "INVALID_STATUS",
		"createdAt": "2024-01-01T00:00:00Z"
	}`)

	result := v.ValidateJob(invalidJob)
	if result.Valid {
		t.Error("Expected invalid, got valid")
	}
}

// TestValidateMessageComplete tests full message validation.
func TestValidateMessageComplete(t *testing.T) {
	v, err := NewValidator()
	if err != nil {
		t.Fatalf("NewValidator failed: %v", err)
	}

	validMessage := []byte(`{
		"contractVersion": "1.0.0",
		"eventType": "job.created",
		"eventId": "550e8400-e29b-41d4-a716-446655440000",
		"occurredAt": "2024-01-01T00:00:00Z",
		"correlationId": "550e8400-e29b-41d4-a716-446655440001",
		"idempotencyKey": "550e8400-e29b-41d4-a716-446655440002",
		"producer": {
			"service": "test-service",
			"instanceId": "instance-1",
			"version": "0.1.0"
		},
		"payload": {"id": "550e8400-e29b-41d4-a716-446655440003", "type": "test", "status": "PENDING", "createdAt": "2024-01-01T00:00:00Z"}
	}`)

	result := v.ValidateMessage(validMessage)
	if !result.Valid {
		t.Errorf("Expected valid, got invalid with errors: %+v", result.Errors)
	}
}

// TestValidationResultStruct tests ValidationResult and ValidationError structs.
func TestValidationResultStruct(t *testing.T) {
	// Test creation of validation result with errors
	result := ValidationResult{
		Valid: false,
		Errors: []ValidationError{
			{Field: "eventId", Message: "required"},
		},
	}

	if result.Valid {
		t.Error("Expected Valid to be false")
	}
	if len(result.Errors) != 1 {
		t.Errorf("Expected 1 error, got %d", len(result.Errors))
	}
	if result.Errors[0].Field != "eventId" {
		t.Errorf("Expected field 'eventId', got '%s'", result.Errors[0].Field)
	}
}
