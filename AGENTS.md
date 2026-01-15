# AGENTS.md - DGuesser Codebase Guide

This document provides guidelines for AI coding agents working in this repository.

## Project Overview

DGuesser is a geography guessing game with:
- **Backend**: Rust workspace (Axum API on port 3001, Socket.IO realtime on port 3002)
- **Frontend**: SvelteKit with Svelte 5, TypeScript, Tailwind CSS (port 5173)
- **Infrastructure**: PostgreSQL + Redis via Docker Compose

## Architecture

```
crates/
  api/        # REST API (Axum)
  realtime/   # Socket.IO server
  core/       # Domain logic (scoring, geo calculations)
  db/         # Database layer (sqlx + PostgreSQL)
  auth/       # Authentication (OAuth, sessions)
  protocol/   # Shared DTOs for API and Socket events
frontend/     # SvelteKit app
```

## Build & Test Commands

### Backend (Rust)
```bash
cargo build                           # Debug build
cargo run -p dguesser-api             # Run API server
cargo run -p dguesser-realtime        # Run realtime server
make dev                              # API with hot reload (requires cargo-watch)
make dev-realtime                     # Realtime with hot reload
```

### Frontend
```bash
cd frontend && bun install            # Install dependencies
cd frontend && bun run dev            # Development server
cd frontend && bun run check          # TypeScript + Svelte type checking
```

### Infrastructure
```bash
make db-up                            # Start PostgreSQL + Redis
make db-down                          # Stop containers
```

## Testing

```bash
cargo test --workspace                # All tests
cargo test -p dguesser-core           # Single crate
cargo test -p dguesser-core test_perfect_score   # Single test by name
cargo test distance                   # Tests matching pattern
cargo test -- --nocapture             # With output
```

## Linting & Formatting

```bash
cargo fmt --all                       # Format code
cargo fmt --all -- --check            # Check formatting
cargo clippy --workspace -- -D warnings   # Lint (warnings as errors)
```

## Rust Code Style

### rustfmt.toml Settings
- Max width: 100 chars, Edition 2021
- Imports grouped: std first, external crates, then local

### Import Order
```rust
use std::net::SocketAddr;           // 1. Standard library
use axum::Router;                   // 2. External crates
use crate::config::Config;          // 3. Local modules
```

### Documentation
```rust
//! Module-level docs at file top

/// Function/struct docs above item
pub fn calculate_score(...) -> u32 { ... }
```

### Error Handling
- `thiserror` for custom error types in libraries
- `anyhow::Result` for application-level errors

```rust
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}
```

### Naming Conventions
- Types: `PascalCase` (GameSession, ApiError)
- Functions: `snake_case` (calculate_score)
- Constants: `SCREAMING_SNAKE_CASE` (MAX_SCORE)
- Modules: `snake_case`

### Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_decreases_with_distance() {
        // descriptive test names
    }
}
```

## Frontend Code Style (Svelte/TypeScript)

### File Structure
```svelte
<script lang="ts">
  // TypeScript with strict mode
</script>

<main class="min-h-screen flex...">
  <!-- Tailwind utility classes only -->
</main>
```

### Stores
```typescript
import { writable } from 'svelte/store';
export const user = writable<User | null>(null);
```

## Common Patterns

### Axum Route Handler
```rust
pub fn router() -> Router<AppState> {
    Router::new().route("/me", get(get_me))
}

async fn get_me(State(state): State<AppState>) -> Result<Json<User>, ApiError> {
    // implementation
}
```

### Config Loading
```rust
dotenvy::dotenv().ok();
let config = Config::from_env()?;
```

### Type Aliases
```rust
pub type DbPool = PgPool;   // Prefer for external types
```

## Key Dependencies

**Backend**: axum, socketioxide, sqlx, tokio, serde, tracing
**Frontend**: svelte 5.x, @sveltejs/kit, tailwindcss 4.x, socket.io-client

## Environment Setup

```bash
cp .env.example .env
```

Required: PostgreSQL (5432), Redis (6379)

## Socket Events

Defined in `crates/protocol/src/socket/events.rs`:
- Client events: `client::JOIN_GAME`, `client::SUBMIT_GUESS`, etc.
- Server events: `server::GAME_STATE`, `server::ROUND_START`, etc.
