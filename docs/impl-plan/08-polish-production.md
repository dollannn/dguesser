# Phase 8: Polish & Production

**Priority:** P2  
**Duration:** 3-5 days  
**Dependencies:** Phase 7 (Game Experience)

## Objectives

- Implement rate limiting
- Add comprehensive error handling
- Set up observability (logging, metrics, tracing)
- Configure production deployment
- Add security hardening
- Performance optimization
- Create admin/moderation tools

## Deliverables

### 8.1 Rate Limiting

**crates/api/src/middleware/rate_limit.rs:**
```rust
use axum::{
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone)]
pub struct RateLimitConfig {
    /// Max requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
    /// Key prefix
    pub prefix: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            prefix: "ratelimit".to_string(),
        }
    }
}

pub async fn rate_limit_middleware<B>(
    State(redis): State<redis::Client>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    config: RateLimitConfig,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let ip = addr.ip().to_string();
    let key = format!("{}:{}:{}", config.prefix, request.uri().path(), ip);

    let mut conn = redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Increment counter
    let count: u32 = conn
        .incr(&key, 1)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set expiry on first request
    if count == 1 {
        let _: () = conn
            .expire(&key, config.window.as_secs() as i64)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if count > config.max_requests {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Rate limit specifically for auth endpoints (stricter)
pub fn auth_rate_limit() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 10,
        window: Duration::from_secs(60),
        prefix: "ratelimit:auth".to_string(),
    }
}

/// Rate limit for game actions
pub fn game_rate_limit() -> RateLimitConfig {
    RateLimitConfig {
        max_requests: 30,
        window: Duration::from_secs(60),
        prefix: "ratelimit:game".to_string(),
    }
}
```

### 8.2 Structured Logging

**crates/api/src/logging.rs:**
```rust
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_logging(json_output: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dguesser=debug,tower_http=debug"));

    if json_output {
        // JSON format for production
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_current_span(true)
                    .with_target(true)
                    .with_thread_ids(true),
            )
            .init();
    } else {
        // Pretty format for development
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_span_events(FmtSpan::CLOSE),
            )
            .init();
    }
}

/// Request ID middleware
pub async fn request_id_middleware<B>(
    request: axum::http::Request<B>,
    next: axum::middleware::Next<B>,
) -> axum::response::Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    let _guard = span.enter();
    
    let mut response = next.run(request).await;
    
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    
    response
}
```

### 8.3 Error Handling Enhancement

**crates/api/src/error.rs (enhanced):**
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
    pub internal_message: Option<String>,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            internal_message: None,
            details: None,
        }
    }

    pub fn with_internal(mut self, msg: impl Into<String>) -> Self {
        self.internal_message = Some(msg.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    // Common errors
    pub fn bad_request(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, msg)
    }

    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required")
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", msg)
    }

    pub fn not_found(resource: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", format!("{} not found", resource))
    }

    pub fn conflict(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, msg)
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "An internal error occurred",
        )
    }

    pub fn rate_limited() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "Too many requests, please slow down",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log internal error details
        if let Some(internal) = &self.internal_message {
            error!(
                code = %self.code,
                status = %self.status.as_u16(),
                internal = %internal,
                "API error"
            );
        }

        let body = ApiErrorResponse {
            code: self.code,
            message: self.message,
            request_id: None, // Would be injected by middleware
            details: self.details,
        };

        (self.status, Json(body)).into_response()
    }
}

// Implement From for common error types
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("Resource"),
            sqlx::Error::Database(db_err) => {
                // Handle unique constraint violations
                if db_err.is_unique_violation() {
                    Self::conflict("DUPLICATE", "Resource already exists")
                } else {
                    Self::internal().with_internal(db_err.to_string())
                }
            }
            _ => Self::internal().with_internal(err.to_string()),
        }
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        Self::internal().with_internal(format!("Redis error: {}", err))
    }
}
```

### 8.4 Health Checks

**crates/api/src/routes/health.rs:**
```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: CheckResult,
    pub redis: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_check = check_database(state.db()).await;
    let redis_check = check_redis(state.redis()).await;

    let overall_status = if db_check.status == "healthy" && redis_check.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if overall_status == StatusCode::OK {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: HealthChecks {
            database: db_check,
            redis: redis_check,
        },
    };

    (overall_status, Json(response))
}

async fn check_database(pool: &sqlx::PgPool) -> CheckResult {
    let start = std::time::Instant::now();
    
    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => CheckResult {
            status: "healthy".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(e) => CheckResult {
            status: "unhealthy".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

async fn check_redis(client: &redis::Client) -> CheckResult {
    let start = std::time::Instant::now();
    
    match client.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(_) => CheckResult {
                    status: "healthy".to_string(),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    error: None,
                },
                Err(e) => CheckResult {
                    status: "unhealthy".to_string(),
                    latency_ms: None,
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => CheckResult {
            status: "unhealthy".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
    }
}

/// Liveness check (simple, for k8s probes)
pub async fn liveness() -> StatusCode {
    StatusCode::OK
}

/// Readiness check (full dependency check)
pub async fn readiness(State(state): State<AppState>) -> StatusCode {
    let (db, redis) = tokio::join!(
        check_database(state.db()),
        check_redis(state.redis())
    );

    if db.status == "healthy" && redis.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
```

### 8.5 Railway Deployment

Railway provides managed infrastructure with built-in Postgres, Redis, and automatic HTTPS.

#### Project Structure

Create a Railway project with 4 services:
- **api** - REST API (from Dockerfile)
- **realtime** - Socket.IO server (from Dockerfile)
- **frontend** - SvelteKit app (from Dockerfile)
- **Postgres** - Railway managed database
- **Redis** - Railway managed Redis

#### Dockerfiles

**Dockerfile.api:**
```dockerfile
# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release
RUN cargo build --release -p api

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/api ./api
COPY migrations ./migrations

# Railway sets PORT env var
ENV PORT=3001
EXPOSE 3001

CMD ["./api"]
```

**Dockerfile.realtime:**
```dockerfile
FROM rust:1.83-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p realtime

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/realtime ./realtime

ENV PORT=3002
EXPOSE 3002

CMD ["./realtime"]
```

**frontend/Dockerfile:**
```dockerfile
FROM node:22-alpine AS builder

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile

COPY . .
RUN pnpm build

FROM node:22-alpine

WORKDIR /app

COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules

ENV NODE_ENV=production
# Railway sets PORT automatically
EXPOSE 3000

CMD ["node", "build"]
```

#### railway.toml (Root Config)

```toml
[build]
builder = "dockerfile"

[deploy]
healthcheckPath = "/health"
healthcheckTimeout = 30
restartPolicyType = "on_failure"
restartPolicyMaxRetries = 3
```

#### Environment Variables

Set these in Railway dashboard for each service:

**api service:**
```bash
DATABASE_URL=${{Postgres.DATABASE_URL}}
REDIS_URL=${{Redis.REDIS_URL}}
FRONTEND_URL=https://your-frontend.up.railway.app
GOOGLE_CLIENT_ID=xxx
GOOGLE_CLIENT_SECRET=xxx
GOOGLE_REDIRECT_URI=https://your-frontend.up.railway.app/auth/callback/google
MICROSOFT_CLIENT_ID=xxx
MICROSOFT_CLIENT_SECRET=xxx
MICROSOFT_REDIRECT_URI=https://your-frontend.up.railway.app/auth/callback/microsoft
SESSION_SECRET=generate-a-64-byte-secret
RUST_LOG=info,dguesser=debug
RUST_ENV=production
```

**realtime service:**
```bash
DATABASE_URL=${{Postgres.DATABASE_URL}}
REDIS_URL=${{Redis.REDIS_URL}}
RUST_LOG=info,dguesser=debug
```

**frontend service:**
```bash
ORIGIN=https://your-frontend.up.railway.app
PUBLIC_API_URL=https://your-api.up.railway.app
PUBLIC_REALTIME_URL=https://your-realtime.up.railway.app
PUBLIC_GOOGLE_MAPS_API_KEY=xxx
```

#### Railway Service Configuration

**API Service:**
- Dockerfile Path: `Dockerfile.api`
- Health Check: `/health`
- Public Networking: Enabled (generates URL)

**Realtime Service:**
- Dockerfile Path: `Dockerfile.realtime`
- Health Check: `/health`
- Public Networking: Enabled (generates URL)

**Frontend Service:**
- Root Directory: `frontend`
- Dockerfile Path: `Dockerfile`
- Public Networking: Enabled (custom domain)

#### Custom Domain Setup

1. Add custom domain in Railway dashboard
2. Configure DNS CNAME to Railway-provided value
3. Railway handles SSL automatically

```
dguesser.com        -> frontend service
api.dguesser.com    -> api service
ws.dguesser.com     -> realtime service
```

#### Railway CLI Deployment

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Link to project
railway link

# Deploy all services
railway up

# View logs
railway logs -s api
railway logs -s realtime
railway logs -s frontend

# Open dashboard
railway open
```

#### Private Networking

Railway services can communicate via private network:

```bash
# In api/realtime, reference other services internally:
# Instead of public URLs, use Railway's private DNS
REALTIME_INTERNAL_URL=http://realtime.railway.internal:3002
```

#### Scaling on Railway

```bash
# Scale via dashboard or CLI
railway service update --replicas 2 -s api
```

For horizontal scaling of realtime service, ensure Redis pub/sub is configured for cross-instance communication.

### 8.6 Railway-Specific Considerations

#### HTTPS & Routing

Railway automatically provides:
- SSL/TLS termination for all public services
- HTTP/2 support
- WebSocket support (no special config needed)
- Automatic certificate renewal

No Nginx/reverse proxy needed - Railway handles this at the edge.

#### CORS Configuration

Since services are on different subdomains, configure CORS properly:

**crates/api/src/routes/mod.rs:**
```rust
use tower_http::cors::{Any, CorsLayer};

pub fn cors_layer(frontend_url: &str) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(frontend_url.parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, COOKIE])
        .allow_credentials(true)  // Required for cookies
        .max_age(Duration::from_secs(3600))
}
```

#### Session Cookies Across Subdomains

For cookies to work across `dguesser.com`, `api.dguesser.com`, etc.:

```rust
// In session config
SessionConfig {
    cookie_name: "dguesser_sid".to_string(),
    domain: Some(".dguesser.com".to_string()), // Note the leading dot
    path: "/".to_string(),
    secure: true,
    same_site: SameSite::None, // Required for cross-subdomain
    ..Default::default()
}
```

#### Railway Health Checks

Railway pings your health endpoint. Ensure fast response:

```rust
// Simple health for Railway (< 30s timeout)
pub async fn health() -> StatusCode {
    StatusCode::OK
}

// Detailed health for monitoring
pub async fn health_detailed(State(state): State<AppState>) -> Json<HealthResponse> {
    // ... full checks
}
```

#### Environment-Based Config

```rust
// Detect Railway environment
pub fn is_railway() -> bool {
    std::env::var("RAILWAY_ENVIRONMENT").is_ok()
}

// Get public URL (Railway provides this)
pub fn get_public_url() -> Option<String> {
    std::env::var("RAILWAY_PUBLIC_DOMAIN").ok()
        .map(|d| format!("https://{}", d))
}
```

#### Logs & Observability

Railway aggregates logs automatically. Use structured JSON logging:

```rust
// In production, output JSON logs
if is_railway() {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
```

View logs in Railway dashboard or CLI:
```bash
railway logs -s api --tail
railway logs -s realtime --tail
```

#### Database Migrations on Deploy

Add a release command to run migrations:

**railway.toml (for api service):**
```toml
[deploy]
startCommand = "./api"
healthcheckPath = "/health"

# Run migrations before starting
[deploy.releaseCommand]
command = "sqlx migrate run"
```

Or handle in application startup (recommended):
```rust
// In main.rs
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .expect("Failed to run migrations");
```

### 8.7 Frontend Production Build

**frontend/Dockerfile.production:**
```dockerfile
FROM node:22-alpine AS builder

WORKDIR /app

COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile

COPY . .
RUN pnpm build

FROM node:22-alpine

WORKDIR /app

COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules

ENV NODE_ENV=production
ENV PORT=3000

EXPOSE 3000

CMD ["node", "build"]
```

### 8.8 Database Backups on Railway

Railway Postgres includes automatic point-in-time backups on Pro plan.

#### Manual Backup via CLI

```bash
# Get connection string
railway variables -s Postgres

# Dump database locally
pg_dump "$DATABASE_URL" > backup_$(date +%Y%m%d).sql

# Or compressed
pg_dump "$DATABASE_URL" | gzip > backup_$(date +%Y%m%d).sql.gz
```

#### Automated Backups with GitHub Actions

**.github/workflows/backup.yml:**
```yaml
name: Database Backup

on:
  schedule:
    - cron: '0 3 * * *'  # Daily at 3 AM
  workflow_dispatch:

jobs:
  backup:
    runs-on: ubuntu-latest
    steps:
      - name: Install PostgreSQL client
        run: sudo apt-get install -y postgresql-client

      - name: Create backup
        env:
          DATABASE_URL: ${{ secrets.DATABASE_URL }}
        run: |
          pg_dump "$DATABASE_URL" | gzip > backup_$(date +%Y%m%d).sql.gz

      - name: Upload to S3/R2
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1
        
      - run: |
          aws s3 cp backup_*.sql.gz s3://dguesser-backups/
```

#### Restore from Backup

```bash
# Download backup
aws s3 cp s3://dguesser-backups/backup_20240115.sql.gz .

# Restore (will drop existing data!)
gunzip -c backup_20240115.sql.gz | psql "$DATABASE_URL"
```

### 8.9 Monitoring on Railway

Railway provides built-in observability:

#### Railway Dashboard Metrics

- CPU & Memory usage per service
- Network I/O
- Request counts (for public services)
- Deploy history and rollbacks

#### Log Aggregation

Railway automatically collects stdout/stderr. Use structured logging:

```rust
// Log with context
tracing::info!(
    user_id = %user.id,
    game_id = %game.id,
    "User joined game"
);

// Log errors with details
tracing::error!(
    error = %e,
    request_id = %req_id,
    "Failed to process request"
);
```

Search logs in Railway dashboard or CLI:
```bash
railway logs -s api | grep "error"
```

#### External Monitoring (Optional)

For advanced monitoring, integrate with external services:

**Axiom (Recommended for Railway):**
```rust
// Add axiom-rs to Cargo.toml
use axiom_rs::Client;

// Send structured logs to Axiom
let client = Client::new()?;
client.ingest("dguesser-logs", vec![event]).await?;
```

**Sentry for Error Tracking:**
```rust
// Add sentry to Cargo.toml
let _guard = sentry::init((
    std::env::var("SENTRY_DSN").ok(),
    sentry::ClientOptions {
        release: sentry::release_name!(),
        environment: Some("production".into()),
        ..Default::default()
    },
));
```

**Uptime Monitoring:**
- Use Railway's built-in health checks
- Add external monitoring: Uptime Robot, Better Uptime, or Checkly

```bash
# External health check URL
https://api.dguesser.com/health
```

#### Custom Metrics Endpoint

Expose Prometheus-compatible metrics:

```rust
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusBuilder;

// In main.rs
let recorder = PrometheusBuilder::new()
    .install_recorder()
    .expect("failed to install recorder");

// Metrics endpoint
async fn metrics() -> String {
    recorder.render()
}

// Add route
.route("/metrics", get(metrics))
```

Then scrape with Grafana Cloud or self-hosted Prometheus.

## Production Checklist

### Security
- [ ] HTTPS enabled (Railway automatic)
- [ ] Session cookies are Secure, HttpOnly, SameSite=None
- [ ] Cookie domain set for cross-subdomain access
- [ ] CORS configured for production domains only
- [ ] Rate limiting enabled (Redis-based)
- [ ] SQL injection prevention (parameterized queries via SQLx)
- [ ] XSS prevention (proper encoding)
- [ ] CSRF protection for state-changing requests
- [ ] Secrets stored in Railway environment variables
- [ ] OAuth redirect URIs locked to production domains

### Performance
- [ ] Database indexes created and verified
- [ ] Connection pooling configured (max 20 per service)
- [ ] Redis caching for sessions and hot data
- [ ] Static assets served with cache headers
- [ ] Response compression enabled
- [ ] Database query optimization (EXPLAIN ANALYZE)

### Reliability
- [ ] Health checks configured in railway.toml
- [ ] Graceful shutdown handling (SIGTERM)
- [ ] Database backups automated (Railway + external)
- [ ] Error monitoring configured (Sentry)
- [ ] Structured JSON logging enabled

### Railway Deployment
- [ ] All services deployed and healthy
- [ ] Custom domains configured
- [ ] Environment variables set correctly
- [ ] Private networking between services
- [ ] Resource limits appropriate
- [ ] Rollback tested

### Operations
- [ ] Dockerfiles optimized (multi-stage builds)
- [ ] Environment variables documented in README
- [ ] Deployment via `railway up` or GitHub integration
- [ ] Rollback procedure: `railway rollback`
- [ ] Database migrations run on deploy

## Railway Deployment Workflow

### Initial Setup

```bash
# 1. Install Railway CLI
npm install -g @railway/cli

# 2. Login to Railway
railway login

# 3. Create new project
railway init

# 4. Add Postgres
railway add --plugin postgresql

# 5. Add Redis
railway add --plugin redis

# 6. Create services
railway service create api
railway service create realtime
railway service create frontend
```

### Deploying Updates

```bash
# Deploy all services
railway up

# Deploy specific service
railway up -s api

# View deployment status
railway status

# View logs
railway logs -s api --tail
railway logs -s realtime --tail

# Open service in browser
railway open -s frontend
```

### Environment Management

```bash
# List variables
railway variables -s api

# Set variable
railway variables set GOOGLE_CLIENT_ID=xxx -s api

# Copy variables between environments
railway variables --kv -s api | railway variables set --from-file - -s api --environment staging
```

### Rollbacks

```bash
# List deployments
railway deployments -s api

# Rollback to previous
railway rollback -s api

# Rollback to specific deployment
railway rollback -s api --deployment <id>
```

### Database Operations

```bash
# Connect to database
railway connect postgresql

# Run migrations manually
railway run sqlx migrate run -s api

# Get connection string
railway variables get DATABASE_URL -s api
```

### GitHub Integration (Recommended)

1. Connect GitHub repo in Railway dashboard
2. Configure auto-deploy on push to `main`
3. Set up preview environments for PRs

```yaml
# .github/workflows/test.yml (runs before Railway deploys)
name: Test
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --workspace
```

## Acceptance Criteria

- [ ] Rate limiting prevents abuse
- [ ] Errors logged with proper context
- [ ] Health checks report accurately
- [ ] Railway deployment works end-to-end
- [ ] SSL/TLS working (automatic)
- [ ] Custom domains configured
- [ ] Backups automated
- [ ] Monitoring/alerting configured

## Post-Launch

After initial launch, consider:

1. **Analytics**: Track user engagement, popular game modes
2. **Leaderboards**: Global and friends leaderboards
3. **Maps**: Add more map/region options
4. **Challenges**: Daily/weekly challenges
5. **Achievements**: Player achievements and badges
6. **Social**: Share results, challenge friends
7. **Mobile**: Native mobile apps
8. **Monetization**: Premium features, custom maps
