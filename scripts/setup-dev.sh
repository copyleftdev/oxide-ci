#!/bin/bash
# Oxide CI Development Environment Setup
# Usage: ./scripts/setup-dev.sh

set -e

echo "🔧 Setting up Oxide CI development environment..."

# Check for required tools
check_command() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 is required but not installed."
        exit 1
    fi
}

check_command docker
check_command cargo

# Create .env if it doesn't exist
if [ ! -f .env ]; then
    echo "📝 Creating .env from .env.example..."
    cp .env.example .env
fi

# Start dependencies
echo "🐳 Starting development dependencies..."
docker compose -f docker-compose.dev.yaml up -d

# Wait for services to be healthy
echo "⏳ Waiting for services to be ready..."
sleep 5

# Check PostgreSQL
echo "🔍 Checking PostgreSQL..."
until docker exec oxide-postgres pg_isready -U oxide > /dev/null 2>&1; do
    echo "  Waiting for PostgreSQL..."
    sleep 2
done
echo "✅ PostgreSQL is ready"

# Check NATS
echo "🔍 Checking NATS..."
until curl -s http://localhost:8222/healthz > /dev/null 2>&1; do
    echo "  Waiting for NATS..."
    sleep 2
done
echo "✅ NATS is ready"

# Check MinIO
echo "🔍 Checking MinIO..."
until curl -s http://localhost:9000/minio/health/live > /dev/null 2>&1; do
    echo "  Waiting for MinIO..."
    sleep 2
done
echo "✅ MinIO is ready"

# Create MinIO bucket for cache
echo "📦 Creating cache bucket in MinIO..."
docker exec oxide-minio mc alias set local http://localhost:9000 oxide oxide_dev_password 2>/dev/null || true
docker exec oxide-minio mc mb local/oxide-cache --ignore-existing 2>/dev/null || true

echo ""
echo "✅ Development environment is ready!"
echo ""
echo "Services:"
echo "  PostgreSQL:  localhost:5432"
echo "  NATS:        localhost:4222 (monitoring: http://localhost:8222)"
echo "  MinIO:       localhost:9000 (console: http://localhost:9001)"
echo "  Jaeger:      http://localhost:16686"
echo ""
echo "To run Oxide services:"
echo "  cargo run -p oxide-api"
echo "  cargo run -p oxide-scheduler"
echo "  cargo run -p oxide-agent"
echo ""
echo "To stop dependencies:"
echo "  docker compose -f docker-compose.dev.yaml down"
