# Phase 1: Project Foundation

**Priority:** P0  
**Duration:** 2-3 days  
**Dependencies:** None

## Objectives

- Set up Rust workspace with proper crate structure
- Configure shared dependencies and feature flags
- Set up development environment (Docker, env files)
- Initialize SvelteKit frontend project
- Establish code quality tooling

## Deliverables

### 1.1 Rust Workspace Setup

Create the workspace `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/db",
    "crates/auth",
    "crates/protocol",
    "crates/api",
    "crates/realtime",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
authors = ["Your Name"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Web framework
axum = { version = "0.8", features = ["macros"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace", "timeout", "limit"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }

# Redis
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }

# Socket.IO
socketioxide = { version = "0.15", features = ["state"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Auth/Crypto
oauth2 = "4"
jsonwebtoken = "9"
argon2 = "0.5"
rand = "0.8"

# Utilities
uuid = { version = "1", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Config
dotenvy = "0.15"
config = "0.14"

# Validation
validator = { version = "0.18", features = ["derive"] }

# Dev dependencies
tokio-test = "0.4"
```

### 1.2 Crate Structure

```bash
crates/
├── core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── game/           # Game rules, scoring
│       │   ├── mod.rs
│       │   ├── scoring.rs
│       │   └── rules.rs
│       ├── geo/            # Geographic calculations
│       │   ├── mod.rs
│       │   └── distance.rs
│       └── error.rs
│
├── db/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pool.rs         # Connection pool setup
│       ├── users.rs        # User queries
│       ├── games.rs        # Game queries
│       ├── sessions.rs     # Session queries
│       └── oauth.rs        # OAuth account queries
│
├── auth/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── session.rs      # Session management
│       ├── oauth/          # OAuth providers
│       │   ├── mod.rs
│       │   ├── google.rs
│       │   └── microsoft.rs
│       └── middleware.rs   # Auth extraction
│
├── protocol/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── api/            # REST DTOs
│       │   ├── mod.rs
│       │   ├── auth.rs
│       │   ├── user.rs
│       │   └── game.rs
│       └── socket/         # Socket.IO events
│           ├── mod.rs
│           ├── events.rs
│           └── payloads.rs
│
├── api/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── state.rs
│       ├── error.rs
│       └── routes/
│           ├── mod.rs
│           ├── auth.rs
│           ├── users.rs
│           └── games.rs
│
└── realtime/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── config.rs
        ├── state.rs
        ├── handlers/       # Socket event handlers
        │   ├── mod.rs
        │   ├── game.rs
        │   └── room.rs
        └── actors/         # Game actors
            ├── mod.rs
            └── game_session.rs
```

### 1.3 Docker Compose for Development

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:17
    ports:
      - "5432:5432"
    environment:
      POSTGRES_USER: dguesser
      POSTGRES_PASSWORD: dguesser_dev
      POSTGRES_DB: dguesser
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U dguesser"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  postgres_data:
  redis_data:
```

### 1.4 Environment Configuration

```bash
# .env.example
# Database
DATABASE_URL=postgres://dguesser:dguesser_dev@localhost:5432/dguesser

# Redis
REDIS_URL=redis://localhost:6379

# Server
API_HOST=0.0.0.0
API_PORT=3001
REALTIME_HOST=0.0.0.0
REALTIME_PORT=3002

# Auth
SESSION_SECRET=change-me-in-production-use-64-bytes-minimum
SESSION_TTL_HOURS=168

# OAuth - Google
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
GOOGLE_REDIRECT_URI=http://localhost:5173/auth/callback/google

# OAuth - Microsoft
MICROSOFT_CLIENT_ID=
MICROSOFT_CLIENT_SECRET=
MICROSOFT_REDIRECT_URI=http://localhost:5173/auth/callback/microsoft

# Frontend URL (for CORS)
FRONTEND_URL=http://localhost:5173

# Logging
RUST_LOG=dguesser=debug,tower_http=debug
```

### 1.5 SvelteKit Project Initialization

```bash
# Initialize SvelteKit in frontend/
cd frontend
pnpm create svelte@latest .
# Select: Skeleton project, TypeScript, ESLint, Prettier

# Key dependencies
pnpm add socket.io-client
pnpm add -D @types/node tailwindcss autoprefixer
```

**Frontend structure:**
```
frontend/
├── src/
│   ├── lib/
│   │   ├── api/           # API client
│   │   ├── socket/        # Socket.IO client
│   │   ├── stores/        # Svelte stores
│   │   └── components/    # UI components
│   ├── routes/
│   │   ├── +layout.svelte
│   │   ├── +page.svelte
│   │   ├── auth/
│   │   ├── play/
│   │   └── game/
│   └── app.html
├── static/
├── svelte.config.js
├── tailwind.config.js
└── package.json
```

### 1.6 Tooling Configuration

**rust-toolchain.toml:**
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

**rustfmt.toml:**
```toml
edition = "2024"
max_width = 100
use_small_heuristics = "Max"
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

**.clippy.toml:**
```toml
cognitive-complexity-threshold = 15
```

**Makefile:**
```makefile
.PHONY: dev db-up db-down migrate test lint fmt

dev:
	cargo watch -x 'run -p api'

dev-realtime:
	cargo watch -x 'run -p realtime'

db-up:
	docker-compose up -d postgres redis

db-down:
	docker-compose down

migrate:
	sqlx migrate run

test:
	cargo test --workspace

lint:
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all
```

## Acceptance Criteria

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` runs (even with no tests)
- [ ] `cargo clippy --workspace` passes
- [ ] Docker containers start successfully
- [ ] SQLx CLI installed and configured
- [ ] SvelteKit dev server starts
- [ ] All crates have placeholder `lib.rs`/`main.rs`

## Technical Notes

### Workspace Dependencies Strategy

Use `[workspace.dependencies]` to ensure all crates use identical versions. This prevents "it compiles in api but not realtime" issues.

Each crate's `Cargo.toml` references workspace deps:
```toml
[dependencies]
tokio.workspace = true
axum.workspace = true
```

### SQLx Offline Mode

For CI, generate query metadata:
```bash
cargo sqlx prepare --workspace
```

This creates `.sqlx/` directory with cached query plans.

## Next Phase

Once foundation is complete, proceed to [Phase 2: Database & Core Domain](./02-database-core.md).
