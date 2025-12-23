"""Tests for the schema_validator module."""
import pytest
from schema_validator import SchemaValidator, validate_message


@pytest.fixture
def validator():
    """Create a validator instance for testing."""
    return SchemaValidator()


class TestSchemaValidator:
    """Tests for the SchemaValidator class."""
    
    def test_valid_event_envelope(self, validator):
        """Test that a valid event envelope passes validation."""
        valid_event = {
            "contractVersion": "1.0.0",
            "eventType": "job.created",
            "eventId": "test-123",
            "occurredAt": "2024-01-01T00:00:00Z",
            "producer": {
                "service": "test",
                "instanceId": "test-1",
                "version": "0.1.0"
            },
            "correlationId": "corr-123",
            "idempotencyKey": "idem-123",
            "payload": {
                "id": "job-123",
                "type": "compute",
                "status": "PENDING",
                "payload": {},
                "createdAt": "2024-01-01T00:00:00Z"
            }
        }
        
        is_valid, error = validator.validate_event_envelope(valid_event)
        assert is_valid is True
        assert error is None
    
    def test_invalid_event_envelope_missing_field(self, validator):
        """Test that missing required fields are detected."""
        invalid_event = {
            "eventType": "job.created",
            # Missing: contractVersion, eventId, occurredAt, etc.
        }
        
        is_valid, error = validator.validate_event_envelope(invalid_event)
        assert is_valid is False
        assert error is not None
        assert "contractVersion" in error.lower() or "required" in error.lower()
    
    def test_validate_message_with_valid_job(self, validator):
        """Test full message validation with valid job payload."""
        valid_message = {
            "contractVersion": "1.0.0",
            "eventType": "job.created",
            "eventId": "test-123",
            "occurredAt": "2024-01-01T00:00:00Z",
            "producer": {
                "service": "gateway",
                "instanceId": "gw-1",
                "version": "0.1.0"
            },
            "correlationId": "corr-456",
            "idempotencyKey": "idem-456",
            "payload": {
                "id": "job-456",
                "type": "compute",
                "status": "PENDING",
                "payload": {"data": "test"},
                "createdAt": "2024-01-01T00:00:00Z"
            }
        }
        
        is_valid, error = validator.validate_message(valid_message)
        assert is_valid is True
        assert error is None
    
    def test_validate_message_with_invalid_payload(self, validator):
        """Test that invalid job payload is detected."""
        invalid_message = {
            "contractVersion": "1.0.0",
            "eventType": "job.created",
            "eventId": "test-123",
            "occurredAt": "2024-01-01T00:00:00Z",
            "producer": {
                "service": "gateway",
                "instanceId": "gw-1",
                "version": "0.1.0"
            },
            "correlationId": "corr-789",
            "idempotencyKey": "idem-789",
            "payload": {
                # Missing required job fields
                "type": "compute"
            }
        }
        
        is_valid, error = validator.validate_message(invalid_message)
        assert is_valid is False
        assert error is not None
        assert "payload" in error.lower()


class TestValidateMessageFunction:
    """Tests for the convenience validate_message function."""
    
    def test_validate_message_singleton(self):
        """Test that the convenience function works."""
        valid_message = {
            "contractVersion": "1.0.0",
            "eventType": "job.completed",
            "eventId": "test-completed",
            "occurredAt": "2024-01-01T00:00:00Z",
            "producer": {
                "service": "processor",
                "instanceId": "proc-1",
                "version": "0.1.0"
            },
            "correlationId": "corr-complete",
            "idempotencyKey": "idem-complete",
            "payload": {
                "id": "job-complete",
                "type": "compute",
                "status": "COMPLETED",
                "payload": {},
                "createdAt": "2024-01-01T00:00:00Z"
            }
        }
        
        is_valid, error = validate_message(valid_message)
        assert is_valid is True
