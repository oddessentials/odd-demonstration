"""
Intentional security bugs for AI review E2E testing.
This file contains vulnerabilities that should be detected by AI code review.
"""
import os
import pickle
import subprocess
import sqlite3


def execute_user_command(user_input: str) -> str:
    """Command injection vulnerability - user input passed directly to shell."""
    result = subprocess.run(user_input, shell=True, capture_output=True, text=True)
    return result.stdout


def unsafe_sql_query(user_id: str) -> list:
    """SQL injection vulnerability - string concatenation in query."""
    conn = sqlite3.connect(":memory:")
    cursor = conn.cursor()
    query = f"SELECT * FROM users WHERE id = '{user_id}'"
    cursor.execute(query)
    return cursor.fetchall()


def deserialize_untrusted(data: bytes) -> object:
    """Insecure deserialization - pickle with untrusted data."""
    return pickle.loads(data)


def hardcoded_credentials():
    """Hardcoded secrets in source code."""
    api_key = "sk-proj-1234567890abcdefghijklmnop"
    db_password = "SuperSecret123!"
    return {"key": api_key, "password": db_password}


def path_traversal(filename: str) -> str:
    """Path traversal vulnerability - no sanitization of user input."""
    base_path = "/var/data/"
    return open(base_path + filename).read()
