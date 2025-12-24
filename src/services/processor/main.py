import pika
import psycopg2
import json
import time
import os
import re
import uuid
import prometheus_client as prom
from schema_validator import validate_message


# Read VERSION file with fail-fast on missing/invalid
VERSION_PATH = os.path.join(os.path.dirname(__file__), 'VERSION')
try:
    with open(VERSION_PATH, 'r') as f:
        SERVICE_VERSION = f.read().strip()
    if not re.match(r'^\d+\.\d+\.\d+$', SERVICE_VERSION):
        raise ValueError(f"Invalid SemVer format: {SERVICE_VERSION}")
except Exception as e:
    print(f"FATAL: Failed to load VERSION file: {e}")
    raise SystemExit(1)

# Configuration
RABBITMQ_URL = os.environ.get('RABBITMQ_URL', 'amqp://guest:guest@rabbitmq:5672')
POSTGRES_URL = os.environ.get('POSTGRES_URL', 'postgresql://admin:password123@postgres:5432/task_db')
QUEUE_NAME = 'jobs.created'
OUT_QUEUE = 'jobs.completed'
DLQ_NAME = 'jobs.failed.validation'  # Dead-letter queue for validation failures

# Metrics setup
JOBS_PROCESSED = prom.Counter('processor_jobs_processed_total', 'Total jobs processed')
JOBS_COMPLETED = prom.Counter('processor_jobs_completed_total', 'Total jobs successfully completed')
JOBS_FAILED = prom.Counter('processor_jobs_failed_total', 'Total jobs failed')
JOBS_VALIDATION_FAILED = prom.Counter('processor_jobs_validation_failed_total', 'Jobs rejected due to validation failure')
PROCESSING_TIME = prom.Histogram('processor_job_processing_seconds', 'Time spent processing job')


def get_correlation_id(event: dict) -> str:
    """Extract correlation ID from event for logging."""
    return event.get('correlationId', 'unknown')


def process_job(ch, method, properties, body):
    JOBS_PROCESSED.inc()
    start_time = time.time()
    correlation_id = 'unknown'
    
    try:
        event = json.loads(body)
        correlation_id = get_correlation_id(event)
        print(f"[{correlation_id}] Received event: {event.get('eventId', 'unknown')}")
        
        # Validate message against schemas
        is_valid, validation_error = validate_message(event)
        if not is_valid:
            JOBS_VALIDATION_FAILED.inc()
            print(f"[{correlation_id}] VALIDATION FAILED: {validation_error}")
            
            # Publish to DLQ with error details
            dlq_message = {
                'original_event': event,
                'validation_error': validation_error,
                'rejected_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
                'correlation_id': correlation_id,
                'service': 'processor',
                'service_version': SERVICE_VERSION
            }
            ch.basic_publish(
                exchange='',
                routing_key=DLQ_NAME,
                body=json.dumps(dlq_message),
                properties=pika.BasicProperties(delivery_mode=2)  # Persistent
            )
            
            # Reject without requeue (already sent to DLQ)
            ch.basic_nack(delivery_tag=method.delivery_tag, requeue=False)
            return
        
        job_data = event['payload']
        job_id = job_data['id']
        
        # Update PostgreSQL
        conn = psycopg2.connect(POSTGRES_URL)
        cur = conn.cursor()
        
        # Upsert job
        cur.execute(
            "INSERT INTO jobs (id, type, status, payload, created_at) VALUES (%s, %s, %s, %s, %s) "
            "ON CONFLICT (id) DO UPDATE SET status = %s, updated_at = %s",
            (job_id, job_data['type'], 'EXECUTING', json.dumps(job_data['payload']), job_data['createdAt'],
             'EXECUTING', 'NOW()')
        )
        conn.commit()
        
        # Simulate work
        print(f"[{correlation_id}] Processing job {job_id}...")
        time.sleep(2)
        
        # Update to completed
        cur.execute("UPDATE jobs SET status = 'COMPLETED', updated_at = 'NOW()' WHERE id = %s", (job_id,))
        conn.commit()
        cur.close()
        conn.close()
        
        # Emit completion event
        completion_event = event.copy()
        completion_event['eventType'] = 'job.completed'
        completion_event['eventId'] = str(uuid.uuid4())
        completion_event['occurredAt'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        completion_event['producer']['service'] = 'processor'
        
        ch.basic_publish(exchange='', routing_key=OUT_QUEUE, body=json.dumps(completion_event))
        ch.basic_ack(delivery_tag=method.delivery_tag)
        JOBS_COMPLETED.inc()
        PROCESSING_TIME.observe(time.time() - start_time)
        print(f"[{correlation_id}] Job {job_id} completed.")
        
    except json.JSONDecodeError as e:
        JOBS_VALIDATION_FAILED.inc()
        print(f"[{correlation_id}] JSON PARSE ERROR: {e}")
        # Can't extract correlation ID from invalid JSON
        ch.basic_nack(delivery_tag=method.delivery_tag, requeue=False)
        
    except Exception as e:
        JOBS_FAILED.inc()
        print(f"[{correlation_id}] ERROR processing job: {e}")
        # Requeue for retry on processing errors (not validation errors)
        ch.basic_nack(delivery_tag=method.delivery_tag, requeue=True)


def main():
    print(f"Processor service starting... version: {SERVICE_VERSION}")
    # Start prometheus metrics server
    prom.start_http_server(8000)
    
    # Initialize DB table if needed (in a real app we'd use migrations)
    while True:
        try:
            conn = psycopg2.connect(POSTGRES_URL)
            cur = conn.cursor()
            cur.execute("""
                CREATE TABLE IF NOT EXISTS jobs (
                    id UUID PRIMARY KEY,
                    type TEXT,
                    status TEXT,
                    payload JSONB,
                    created_at TIMESTAMP,
                    updated_at TIMESTAMP DEFAULT NOW()
                )
            """)
            conn.commit()
            cur.close()
            conn.close()
            break
        except Exception as e:
            print(f"Waiting for DB... {e}")
            time.sleep(5)

    params = pika.URLParameters(RABBITMQ_URL)
    connection = pika.BlockingConnection(params)
    channel = connection.channel()
    
    # Declare queues
    channel.queue_declare(queue=QUEUE_NAME, durable=True)
    channel.queue_declare(queue=OUT_QUEUE, durable=True)
    channel.queue_declare(queue=DLQ_NAME, durable=True)  # Dead-letter queue
    
    channel.basic_qos(prefetch_count=1)
    channel.basic_consume(queue=QUEUE_NAME, on_message_callback=process_job)
    
    print(f'Waiting for jobs. DLQ enabled: {DLQ_NAME}')
    print('To exit press CTRL+C')
    channel.start_consuming()


if __name__ == '__main__':
    main()

