"""
Schema validation module for the processor service.

Validates incoming messages against JSON schemas to ensure contract compliance.
Invalid messages are rejected to the dead-letter queue with correlation ID logging.
"""
import json
import os
from pathlib import Path
from typing import Tuple, Optional, Dict, Any
from jsonschema import validate, ValidationError, Draft7Validator


class SchemaValidator:
    """Validates messages against JSON schemas with caching."""
    
    def __init__(self, schemas_dir: Optional[str] = None):
        """
        Initialize the validator with a schemas directory.
        
        Args:
            schemas_dir: Path to schemas directory. Defaults to ./schemas or CONTRACTS_PATH env.
        """
        if schemas_dir:
            self.schemas_dir = Path(schemas_dir)
        else:
            # Check environment variable first, then local schemas dir
            contracts_path = os.environ.get('CONTRACTS_PATH')
            if contracts_path:
                self.schemas_dir = Path(contracts_path) / 'schemas'
            else:
                self.schemas_dir = Path(__file__).parent / 'schemas'
        
        self._cache: Dict[str, dict] = {}
        self._validators: Dict[str, Draft7Validator] = {}
    
    def _load_schema(self, name: str) -> dict:
        """Load and cache a schema by name."""
        if name not in self._cache:
            schema_path = self.schemas_dir / f"{name}.json"
            if not schema_path.exists():
                raise FileNotFoundError(f"Schema not found: {schema_path}")
            
            with open(schema_path, 'r', encoding='utf-8') as f:
                self._cache[name] = json.load(f)
            
            # Create compiled validator for performance
            self._validators[name] = Draft7Validator(self._cache[name])
        
        return self._cache[name]
    
    def _get_validator(self, name: str) -> Draft7Validator:
        """Get compiled validator for a schema."""
        if name not in self._validators:
            self._load_schema(name)
        return self._validators[name]
    
    def validate_event_envelope(self, event: dict) -> Tuple[bool, Optional[str]]:
        """
        Validate an event envelope against the event-envelope schema.
        
        Args:
            event: The event dictionary to validate.
            
        Returns:
            Tuple of (is_valid, error_message or None)
        """
        try:
            validator = self._get_validator('event-envelope')
            errors = list(validator.iter_errors(event))
            if errors:
                error_messages = [f"{e.json_path}: {e.message}" for e in errors[:3]]
                return False, "; ".join(error_messages)
            return True, None
        except FileNotFoundError as e:
            return False, str(e)
        except Exception as e:
            return False, f"Validation error: {str(e)}"
    
    def validate_job(self, job: dict) -> Tuple[bool, Optional[str]]:
        """
        Validate a job payload against the job schema.
        
        Args:
            job: The job payload dictionary to validate.
            
        Returns:
            Tuple of (is_valid, error_message or None)
        """
        try:
            validator = self._get_validator('job')
            errors = list(validator.iter_errors(job))
            if errors:
                error_messages = [f"{e.json_path}: {e.message}" for e in errors[:3]]
                return False, "; ".join(error_messages)
            return True, None
        except FileNotFoundError as e:
            return False, str(e)
        except Exception as e:
            return False, f"Validation error: {str(e)}"
    
    def validate_message(self, event: dict) -> Tuple[bool, Optional[str]]:
        """
        Validate a complete message (envelope + payload).
        
        Args:
            event: The complete event including envelope and payload.
            
        Returns:
            Tuple of (is_valid, error_message or None)
        """
        # First validate the envelope
        is_valid, error = self.validate_event_envelope(event)
        if not is_valid:
            return False, f"Envelope validation failed: {error}"
        
        # Then validate the payload if it's a job event
        event_type = event.get('eventType', '')
        if event_type.startswith('job.'):
            payload = event.get('payload', {})
            is_valid, error = self.validate_job(payload)
            if not is_valid:
                return False, f"Payload validation failed: {error}"
        
        return True, None


# Singleton instance for convenience
_default_validator: Optional[SchemaValidator] = None


def get_validator() -> SchemaValidator:
    """Get the default validator instance (singleton)."""
    global _default_validator
    if _default_validator is None:
        _default_validator = SchemaValidator()
    return _default_validator


def validate_message(event: dict) -> Tuple[bool, Optional[str]]:
    """Convenience function to validate a message using the default validator."""
    return get_validator().validate_message(event)
