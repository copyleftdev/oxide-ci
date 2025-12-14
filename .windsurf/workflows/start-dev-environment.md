# Start Development Environment

Workflow for setting up the local development environment.

## Steps

1. **Check Docker is running**
   ```bash
   docker info > /dev/null 2>&1 && echo "Docker is running" || echo "Start Docker first"
   ```

2. **Start dependencies** (PostgreSQL, NATS, MinIO)
   ```bash
   docker compose -f docker-compose.dev.yaml up -d
   ```

3. **Wait for services to be healthy**
   ```bash
   # Wait for PostgreSQL
   until docker compose -f docker-compose.dev.yaml exec -T postgres pg_isready; do
     echo "Waiting for PostgreSQL..."
     sleep 2
   done
   
   # Wait for NATS
   until curl -s http://localhost:8222/healthz > /dev/null; do
     echo "Waiting for NATS..."
     sleep 2
   done
   ```

4. **Run database migrations** (if oxide-db is implemented)
   ```bash
   cargo sqlx migrate run --database-url postgres://oxide:oxide@localhost:5432/oxide
   ```

5. **Verify connections**
   ```bash
   # Test PostgreSQL
   psql postgres://oxide:oxide@localhost:5432/oxide -c "SELECT 1"
   
   # Test NATS
   curl http://localhost:8222/varz
   ```

6. **Build the project**
   ```bash
   cargo build --workspace
   ```

7. **Run the API server** (in one terminal)
   ```bash
   cargo run -p oxide-api
   ```

8. **Run an agent** (in another terminal)
   ```bash
   cargo run -p oxide-agent
   ```

9. **Verify everything is working**
   ```bash
   curl http://localhost:8080/health
   ```

10. **When done, stop services**
    ```bash
    docker compose -f docker-compose.dev.yaml down
    ```
