"""Pytest configuration for processor tests.

Mocks external dependencies (pika, psycopg2, prometheus_client) at module level
to allow testing main.py without installed dependencies.
"""
import sys
from unittest.mock import MagicMock, Mock


# Create mock modules for external dependencies BEFORE any test imports main
class MockPrometheusCounter:
    """Mock Prometheus Counter."""
    def __init__(self, *args, **kwargs):
        self._value = MagicMock()
        self._value.get.return_value = 0
    
    def inc(self):
        current = self._value.get()
        self._value.get.return_value = current + 1


class MockPrometheusHistogram:
    """Mock Prometheus Histogram."""
    def __init__(self, *args, **kwargs):
        pass
    
    def observe(self, value):
        pass


class MockPrometheusClient:
    """Mock prometheus_client module."""
    Counter = MockPrometheusCounter
    Histogram = MockPrometheusHistogram
    
    @staticmethod
    def start_http_server(port):
        pass


class MockPikaBasicProperties:
    """Mock pika.BasicProperties."""
    def __init__(self, delivery_mode=None):
        self.delivery_mode = delivery_mode


class MockPika:
    """Mock pika module."""
    BasicProperties = MockPikaBasicProperties
    
    @staticmethod
    def URLParameters(url):
        return MagicMock()
    
    @staticmethod
    def BlockingConnection(params):
        conn = MagicMock()
        channel = MagicMock()
        conn.channel.return_value = channel
        return conn


class MockPsycopg2:
    """Mock psycopg2 module."""
    @staticmethod
    def connect(url):
        conn = MagicMock()
        cursor = MagicMock()
        conn.cursor.return_value = cursor
        return conn


# Install mocks BEFORE importing main
if 'prometheus_client' not in sys.modules:
    sys.modules['prometheus_client'] = MockPrometheusClient()
if 'pika' not in sys.modules:
    sys.modules['pika'] = MockPika()
if 'psycopg2' not in sys.modules:
    sys.modules['psycopg2'] = MockPsycopg2()
