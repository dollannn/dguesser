# AGENTS.md - DGuesser Codebase Guide

Guidelines for AI coding agents working in this repository.

## Project Overview

DGuesser is a geography guessing game with:
- **Backend**: Rust workspace (Axum API :3001, Socket.IO realtime :3002)
- **Frontend**: SvelteKit 5, TypeScript, Tailwind CSS 4 (:5173)
- **Infrastructure**: PostgreSQL + Redis via Docker Compose

## Architecture

```
crates/
  api/        # REST API (Axum) - routes, middleware, error handling
  realtime/   # Socket.IO server for multiplayer
  core/       # Domain logic (scoring, geo calculations, ID generation)
  db/         # Database layer (sqlx + PostgreSQL)
  auth/       # Authentication (OAuth, sessions)
  protocol/   # Shared DTOs for API and Socket events
frontend/     # SvelteKit app with shadcn-svelte components
```

## Build & Run Commands

```bash
cargo build --workspace            # Build all crates
just dev                           # API with hot reload
just devr                          # Realtime with hot reload
just deva                          # Both servers
just db-up && just migrate         # Start DB + run migrations

cd frontend && bun install && bun run dev   # Frontend dev server
```

## Testing

```bash
cargo test --workspace                          # All tests
cargo test -p dguesser-core                     # Single crate
cargo test -p dguesser-api test_bad_request    # Single test by name
cargo test distance                             # Tests matching pattern
cargo test -- --nocapture                       # Show stdout/stderr
```

## Linting & Formatting

```bash
cargo fmt --all                       # Format Rust code
cargo fmt --all -- --check            # Check formatting (CI)
cargo clippy --workspace -- -D warnings  # Lint with warnings as errors
cd frontend && bun run check          # TypeScript + Svelte type checking
```

## Rust Code Style

### rustfmt.toml: `max_width = 100`, `edition = "2024"`, `group_imports = "StdExternalCrate"`

### Import Order
```rust
use std::net::SocketAddr;           // 1. Standard library
use axum::{Json, Router};           // 2. External crates
use crate::error::ApiError;         // 3. Local modules
```

### Module Documentation
```rust
//! Module-level docs at file top (use //! not //)

/// Function/struct docs above item
pub fn calculate_score() -> u32 { ... }
```

### Error Handling
- `thiserror` for custom error types in library crates
- `anyhow::Result` for application-level errors in binaries
- Implement `From<SourceError>` for automatic error conversion

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
}
```

### Naming: Types `PascalCase`, functions `snake_case`, constants `SCREAMING_SNAKE`, modules `snake_case`

### Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange, Act, Assert pattern
    }
}
```

### Axum Route Handler Pattern
```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/resource", get(get_handler))
        .route("/resource", post(create_handler))
}

async fn get_handler(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Response>, ApiError> { ... }
```

## Frontend Code Style (Svelte 5 / TypeScript)

### Component Structure
```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button';
  import type { User } from '$lib/api/auth';

  let loading = $state(false);  // Svelte 5 runes
</script>

<main class="container mx-auto px-4">
  <!-- Tailwind utility classes only -->
</main>
```

### Import Order: 1) SvelteKit (`$app/*`), 2) External libraries, 3) Local (`$lib/*`)

### Store Pattern
```typescript
function createStore() {
  const { subscribe, set, update } = writable<State>(initialState);
  return { subscribe, async action() { /* ... */ } };
}
export const store = createStore();
export const derived$ = derived(store, ($s) => $s.value);
```

### API Client Usage
```typescript
import { api, ApiClientError } from '$lib/api/client';
try {
  const result = await api.post<Response>('/endpoint', body);
} catch (e) {
  if (e instanceof ApiClientError) console.error(e.code, e.message);
}
```

## Key Dependencies

**Backend**: axum 0.8, socketioxide 0.15, sqlx 0.8, tokio, serde, tracing, utoipa
**Frontend**: svelte 5.x, @sveltejs/kit 2.x, tailwindcss 4.x, bits-ui, socket.io-client

## Environment Setup

```bash
cp .env.example .env
just db-up      # Start PostgreSQL (5432) + Redis (6379)
just migrate    # Run migrations
```

## Socket Events

Defined in `crates/protocol/src/socket/events.rs`:
- Client → Server: `JOIN_GAME`, `SUBMIT_GUESS`, `LEAVE_GAME`
- Server → Client: `GAME_STATE`, `ROUND_START`, `ROUND_END`, `GAME_END`
