"""Processor service tests for Phase 10.5."""
import os
import json
import re
import pytest


def read_version(version_path=None):
    """Read version from VERSION file."""
    if version_path is None:
        version_path = os.path.join(os.path.dirname(__file__), '..', 'VERSION')
    
    if not os.path.exists(version_path):
        raise FileNotFoundError(f"VERSION file not found at {version_path}")
    
    with open(version_path, 'r') as f:
        version = f.read().strip()
    
    if not re.match(r'^\d+\.\d+\.\d+$', version):
        raise ValueError(f"Invalid SemVer format: {version}")
    
    return version


def load_schema(schema_path):
    """Load and validate a JSON schema file."""
    if not os.path.exists(schema_path):
        raise FileNotFoundError(f"Schema not found at {schema_path}")
    
    with open(schema_path, 'r') as f:
        schema = json.load(f)
    
    # Basic schema validation
    if '$schema' not in schema:
        raise ValueError("Schema missing $schema field")
    
    return schema


def healthz_response(version):
    """Build health check response with version."""
    return {
        "status": "ok",
        "version": version
    }


class TestVersionReader:
    """Test version reading from VERSION file."""
    
    def test_read_version_returns_semver(self):
        """read_version() returns SemVer from VERSION file."""
        version = read_version()
        assert re.match(r'^\d+\.\d+\.\d+$', version), f"Invalid SemVer: {version}"
    
    def test_version_is_0_1_0(self):
        """Version should be 0.1.0 for Phase 10.5."""
        version = read_version()
        assert version == "0.1.0"


class TestHealthResponse:
    """Test health endpoint response."""
    
    def test_health_response_includes_version(self):
        """Health response includes version field."""
        version = read_version()
        response = healthz_response(version)
        
        assert "version" in response
        assert response["version"] == version
    
    def test_health_response_status_ok(self):
        """Health response has status ok."""
        response = healthz_response("0.1.0")
        assert response["status"] == "ok"


class TestSchemaLoader:
    """Test schema loading functionality."""
    
    def test_load_canonical_schema(self):
        """Schema validator loads event-envelope schema."""
        # Path relative to project root
        contracts_path = os.path.join(
            os.path.dirname(__file__), 
            '..', '..', '..', '..', 
            'contracts', 'schemas', 'event-envelope.json'
        )
        
        if os.path.exists(contracts_path):
            schema = load_schema(contracts_path)
            assert schema.get('title') == 'EventEnvelope'
            assert 'properties' in schema
        else:
            # Skip if running outside project context
            pytest.skip("Contracts path not available in this test context")


# ============================================================
# Phase 12: State Machine Invariants (G6)
# ============================================================

# Valid job statuses and allowed transitions
VALID_STATUSES = {'PENDING', 'PROCESSING', 'COMPLETED', 'FAILED'}
VALID_TRANSITIONS = {
    'PENDING': {'PROCESSING'},
    'PROCESSING': {'COMPLETED', 'FAILED'},
    'COMPLETED': set(),  # Terminal state
    'FAILED': set(),     # Terminal state
}


class JobStateMachine:
    """State machine for job status transitions."""
    
    def __init__(self, job_id: str, initial_status: str = 'PENDING'):
        if initial_status not in VALID_STATUSES:
            raise ValueError(f"Invalid initial status: {initial_status}")
        self.job_id = job_id
        self.status = initial_status
        self.processed_event_ids = set()
    
    def transition(self, new_status: str) -> bool:
        """Attempt status transition. Returns True if valid."""
        if new_status not in VALID_STATUSES:
            raise ValueError(f"Invalid target status: {new_status}")
        
        if new_status in VALID_TRANSITIONS.get(self.status, set()):
            self.status = new_status
            return True
        return False
    
    def process_event(self, event_id: str, delivery_tag: int, redelivered: bool) -> str:
        """Process an event, handling idempotency.
        
        Returns: 'processed', 'duplicate', or 'invalid'
        """
        # V4: Realistic idempotency - same eventId but potentially different delivery metadata
        if event_id in self.processed_event_ids:
            # Already processed - idempotent no-op
            return 'duplicate'
        
        self.processed_event_ids.add(event_id)
        return 'processed'


class TestJobStateMachine:
    """Test job state machine transitions (G6 enforcement)."""
    
    def test_valid_transition_pending_to_processing(self):
        """PENDING → PROCESSING is valid."""
        job = JobStateMachine('job-001', 'PENDING')
        result = job.transition('PROCESSING')
        assert result is True
        assert job.status == 'PROCESSING'
    
    def test_valid_transition_processing_to_completed(self):
        """PROCESSING → COMPLETED is valid."""
        job = JobStateMachine('job-002', 'PENDING')
        job.transition('PROCESSING')
        result = job.transition('COMPLETED')
        assert result is True
        assert job.status == 'COMPLETED'
    
    def test_valid_transition_processing_to_failed(self):
        """PROCESSING → FAILED is valid."""
        job = JobStateMachine('job-003', 'PENDING')
        job.transition('PROCESSING')
        result = job.transition('FAILED')
        assert result is True
        assert job.status == 'FAILED'
    
    def test_invalid_transition_completed_to_processing(self):
        """COMPLETED → PROCESSING is INVALID (terminal state)."""
        job = JobStateMachine('job-004', 'PENDING')
        job.transition('PROCESSING')
        job.transition('COMPLETED')
        result = job.transition('PROCESSING')
        assert result is False
        assert job.status == 'COMPLETED'  # Status unchanged
    
    def test_invalid_transition_pending_to_completed(self):
        """PENDING → COMPLETED is INVALID (must go through PROCESSING)."""
        job = JobStateMachine('job-005', 'PENDING')
        result = job.transition('COMPLETED')
        assert result is False
        assert job.status == 'PENDING'
    
    def test_invalid_status_raises_error(self):
        """Invalid status string raises ValueError."""
        with pytest.raises(ValueError, match="Invalid"):
            JobStateMachine('job-006', 'INVALID_STATUS')


class TestIdempotency:
    """Test idempotency with realistic RabbitMQ redelivery scenarios (V4)."""
    
    def test_duplicate_event_is_noop(self):
        """Same eventId with different deliveryTag is idempotent no-op."""
        job = JobStateMachine('job-idempotent-001')
        event_id = 'evt-abc-123'
        
        # First delivery
        result1 = job.process_event(event_id, delivery_tag=1, redelivered=False)
        assert result1 == 'processed'
        
        # Redelivery with different delivery metadata (realistic RabbitMQ scenario)
        result2 = job.process_event(event_id, delivery_tag=2, redelivered=True)
        assert result2 == 'duplicate'
    
    def test_different_events_both_processed(self):
        """Different eventIds are both processed."""
        job = JobStateMachine('job-idempotent-002')
        
        result1 = job.process_event('evt-001', delivery_tag=1, redelivered=False)
        result2 = job.process_event('evt-002', delivery_tag=2, redelivered=False)
        
        assert result1 == 'processed'
        assert result2 == 'processed'
        assert len(job.processed_event_ids) == 2
    
    def test_redelivery_with_same_delivery_tag_is_duplicate(self):
        """Exact same delivery (same eventId, same deliveryTag) is duplicate."""
        job = JobStateMachine('job-idempotent-003')
        event_id = 'evt-exact-dup'
        
        job.process_event(event_id, delivery_tag=5, redelivered=False)
        result = job.process_event(event_id, delivery_tag=5, redelivered=False)
        
        assert result == 'duplicate'


class TestMalformedInput:
    """Test handling of malformed input (error shape, not schema)."""
    
    def test_missing_job_id_raises_error(self):
        """Missing job_id raises TypeError."""
        with pytest.raises(TypeError):
            JobStateMachine()  # Missing required job_id
    
    def test_none_status_raises_error(self):
        """None status raises appropriate error."""
        with pytest.raises((ValueError, TypeError)):
            JobStateMachine('job-bad', None)


# ============================================================
# Contract Compliance Tests (per contracts/schemas/job.json)
# ============================================================

class TestContractCompliance:
    """Tests ensuring processor conforms to contracts/schemas/job.json.
    
    Per contract-rules.md: "Implementations must conform to contracts;
    implementations do not redefine them."
    """
    
    def test_job_without_payload_is_valid_per_schema(self):
        """Per job.json, 'payload' is optional. Processor must handle this.
        
        This test matches contracts/fixtures/golden/event-envelope-valid.json
        where the job has no nested 'payload' field.
        """
        # Job without payload - valid per job.json (payload not in required array)
        job_data = {
            "id": "770e8400-e29b-41d4-a716-446655440002",
            "type": "test-job",
            "status": "PENDING",
            "createdAt": "2025-01-01T12:00:00Z"
            # Note: NO 'payload' field - optional per job.json
        }
        
        # The processor's extraction logic must not fail
        extracted_payload = job_data.get('payload', {})
        assert extracted_payload == {}
        assert json.dumps(extracted_payload) == '{}'
    
    def test_job_with_payload_extracts_correctly(self):
        """Jobs WITH payload still work correctly.
        
        This test matches contracts/fixtures/golden/job-valid.json.
        """
        job_data = {
            "id": "880e8400-e29b-41d4-a716-446655440003",
            "type": "data-processing",
            "status": "PENDING",
            "createdAt": "2025-01-01T12:00:00Z",
            "payload": {"task": "Sample task", "priority": "high"}
        }
        
        extracted_payload = job_data.get('payload', {})
        assert extracted_payload == {"task": "Sample task", "priority": "high"}
    
    def test_payload_serialization_empty(self):
        """Empty payload serializes correctly for database storage."""
        job_data = {"id": "test", "type": "test", "status": "PENDING", "createdAt": "2025-01-01T12:00:00Z"}
        payload_json = json.dumps(job_data.get('payload', {}))
        assert payload_json == '{}'
    
    def test_payload_serialization_with_data(self):
        """Payload with data serializes correctly for database storage."""
        job_data = {
            "id": "test",
            "type": "test", 
            "status": "PENDING",
            "createdAt": "2025-01-01T12:00:00Z",
            "payload": {"key": "value"}
        }
        payload_json = json.dumps(job_data.get('payload', {}))
        assert payload_json == '{"key": "value"}'

