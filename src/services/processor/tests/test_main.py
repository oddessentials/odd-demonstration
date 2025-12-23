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
