import json
import jsonschema
import sys
import os

def validate(schema_path, data_path):
    with open(schema_path, 'r') as f:
        schema = json.load(f)
    
    with open(data_path, 'r') as f:
        data = json.load(f)
    
    try:
        jsonschema.validate(instance=data, schema=schema)
        print(f"Validation successful for {data_path} against {schema_path}")
    except jsonschema.exceptions.ValidationError as e:
        print(f"Validation failed: {e.message}")
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python test-contracts.py <schema_path> <data_path>")
        sys.exit(1)
    validate(sys.argv[1], sys.argv[2])
