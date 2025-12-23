# Web Interface

## Purpose
Static web dashboard for the Distributed Task Observatory. Provides a browser-based UI for viewing job status and system health, served via NGINX.

## Run Locally
```bash
cd src/interfaces/web
python -m http.server 8000        # Simple local server
# Or open index.html directly in browser
```

## Test
No automated tests (static HTML).

## Build / Container
```bash
docker build -t web:local .
docker run -p 80:80 web:local
```

## Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| N/A | - | - | Static content, no runtime configuration |

> **Note:** API endpoints are configured in `index.html`. Update URLs if deploying to non-default locations.
