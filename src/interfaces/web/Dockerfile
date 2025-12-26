# ODTO Web Terminal Dockerfile
# Multi-stage build: bundle with esbuild, serve with nginx

# Stage 1: Build
FROM node:20-alpine AS builder
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY . ./
RUN npm run build

# Stage 2: Serve
FROM nginx:alpine
COPY --from=builder /app/dist/bundle.js /usr/share/nginx/html/
COPY --from=builder /app/dist/bundle.css /usr/share/nginx/html/
COPY --from=builder /app/index.html /usr/share/nginx/html/
COPY --from=builder /app/styles.css /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Expose HTTP port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
    CMD wget -q --spider http://localhost/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
