package validator

import (
	"embed"
	"encoding/json"
	"fmt"
	"sync"

	"github.com/xeipuuv/gojsonschema"
)

//go:embed schemas/*.json
var schemasFS embed.FS

// ValidationError represents a schema validation failure
type ValidationError struct {
	Field   string `json:"field"`
	Message string `json:"message"`
}

// ValidationResult contains the result of schema validation
type ValidationResult struct {
	Valid  bool              `json:"valid"`
	Errors []ValidationError `json:"errors,omitempty"`
}

// Validator validates messages against JSON schemas
type Validator struct {
	schemas map[string]*gojsonschema.Schema
	mu      sync.RWMutex
}

// NewValidator creates a new schema validator
func NewValidator() (*Validator, error) {
	v := &Validator{
		schemas: make(map[string]*gojsonschema.Schema),
	}

	// Pre-load schemas
	schemaNames := []string{"event-envelope", "job"}
	for _, name := range schemaNames {
		if err := v.loadSchema(name); err != nil {
			return nil, fmt.Errorf("failed to load schema %s: %w", name, err)
		}
	}

	return v, nil
}

func (v *Validator) loadSchema(name string) error {
	data, err := schemasFS.ReadFile(fmt.Sprintf("schemas/%s.json", name))
	if err != nil {
		return fmt.Errorf("failed to read schema file: %w", err)
	}

	loader := gojsonschema.NewBytesLoader(data)
	schema, err := gojsonschema.NewSchema(loader)
	if err != nil {
		return fmt.Errorf("failed to compile schema: %w", err)
	}

	v.mu.Lock()
	v.schemas[name] = schema
	v.mu.Unlock()

	return nil
}

// ValidateEventEnvelope validates an event envelope against the schema
func (v *Validator) ValidateEventEnvelope(event []byte) ValidationResult {
	return v.validate("event-envelope", event)
}

// ValidateJob validates a job payload against the schema
func (v *Validator) ValidateJob(job []byte) ValidationResult {
	return v.validate("job", job)
}

// ValidateMessage validates a complete message (envelope + payload)
func (v *Validator) ValidateMessage(message []byte) ValidationResult {
	// First validate envelope
	result := v.ValidateEventEnvelope(message)
	if !result.Valid {
		return result
	}

	// Extract payload and validate if it's a job event
	var envelope struct {
		EventType string                 `json:"eventType"`
		Payload   map[string]interface{} `json:"payload"`
	}
	if err := json.Unmarshal(message, &envelope); err != nil {
		return ValidationResult{
			Valid: false,
			Errors: []ValidationError{{
				Field:   "$",
				Message: fmt.Sprintf("failed to parse message: %v", err),
			}},
		}
	}

	// Validate payload for job events
	if len(envelope.EventType) > 4 && envelope.EventType[:4] == "job." {
		payloadBytes, err := json.Marshal(envelope.Payload)
		if err != nil {
			return ValidationResult{
				Valid: false,
				Errors: []ValidationError{{
					Field:   "$.payload",
					Message: fmt.Sprintf("failed to marshal payload: %v", err),
				}},
			}
		}

		payloadResult := v.ValidateJob(payloadBytes)
		if !payloadResult.Valid {
			// Prefix errors with payload path
			for i := range payloadResult.Errors {
				payloadResult.Errors[i].Field = "$.payload." + payloadResult.Errors[i].Field
			}
			return payloadResult
		}
	}

	return ValidationResult{Valid: true}
}

func (v *Validator) validate(schemaName string, data []byte) ValidationResult {
	v.mu.RLock()
	schema, ok := v.schemas[schemaName]
	v.mu.RUnlock()

	if !ok {
		return ValidationResult{
			Valid: false,
			Errors: []ValidationError{{
				Field:   "$",
				Message: fmt.Sprintf("schema %s not found", schemaName),
			}},
		}
	}

	documentLoader := gojsonschema.NewBytesLoader(data)
	result, err := schema.Validate(documentLoader)
	if err != nil {
		return ValidationResult{
			Valid: false,
			Errors: []ValidationError{{
				Field:   "$",
				Message: fmt.Sprintf("validation error: %v", err),
			}},
		}
	}

	if !result.Valid() {
		errors := make([]ValidationError, 0, len(result.Errors()))
		for _, err := range result.Errors() {
			errors = append(errors, ValidationError{
				Field:   err.Field(),
				Message: err.Description(),
			})
		}
		return ValidationResult{Valid: false, Errors: errors}
	}

	return ValidationResult{Valid: true}
}

// GetCorrelationID extracts the correlation ID from an event for logging
func GetCorrelationID(message []byte) string {
	var envelope struct {
		CorrelationID string `json:"correlationId"`
	}
	if err := json.Unmarshal(message, &envelope); err != nil {
		return "unknown"
	}
	if envelope.CorrelationID == "" {
		return "unknown"
	}
	return envelope.CorrelationID
}
