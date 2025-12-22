import pika
import psycopg2
import json
import time
import os
import uuid
import prometheus_client as prom
from jsonschema import validate


# Configuration
RABBITMQ_URL = os.environ.get('RABBITMQ_URL', 'amqp://guest:guest@rabbitmq:5672')
POSTGRES_URL = os.environ.get('POSTGRES_URL', 'postgresql://admin:password123@postgres:5432/task_db')
QUEUE_NAME = 'jobs.created'
OUT_QUEUE = 'jobs.completed'

# Metrics setup
JOBS_PROCESSED = prom.Counter('processor_jobs_processed_total', 'Total jobs processed')
JOBS_COMPLETED = prom.Counter('processor_jobs_completed_total', 'Total jobs successfully completed')
JOBS_FAILED = prom.Counter('processor_jobs_failed_total', 'Total jobs failed')
PROCESSING_TIME = prom.Histogram('processor_job_processing_seconds', 'Time spent processing job')


# Load schema
CONTRACTS_PATH = os.environ.get('CONTRACTS_PATH', '/app/contracts')
schema_file = os.path.join(CONTRACTS_PATH, 'schemas/event-envelope.json')

with open(schema_file, 'r') as f:
    event_schema = json.load(f)


def process_job(ch, method, properties, body):
    JOBS_PROCESSED.inc()
    start_time = time.time()
    try:

        event = json.loads(body)
        print(f"Received event: {event['eventId']}")
        
        # Validate event
        validate(instance=event, schema=event_schema)
        
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
        print(f"Processing job {job_id}...")
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
        print(f"Job {job_id} completed.")
        
    except Exception as e:
        JOBS_FAILED.inc()
        print(f"Error processing job: {e}")

        # In a real system, we'd handle retries/DLQ here
        ch.basic_nack(delivery_tag=method.delivery_tag, requeue=False)

def main():
    print("Processor service starting...")
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
    
    channel.queue_declare(queue=QUEUE_NAME, durable=True)
    channel.queue_declare(queue=OUT_QUEUE, durable=True)
    
    channel.basic_qos(prefetch_count=1)
    channel.basic_consume(queue=QUEUE_NAME, on_message_callback=process_job)
    
    print('Waiting for jobs. To exit press CTRL+C')
    channel.start_consuming()

if __name__ == '__main__':
    main()
