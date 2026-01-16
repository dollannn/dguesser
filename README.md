# DGuesser

A geography guessing game with multiplayer support.

## Tech Stack

- **Backend**: Rust (Axum REST API + Socket.IO realtime)
- **Frontend**: SvelteKit 5, TypeScript, Tailwind CSS 4
- **Database**: PostgreSQL + Redis

## Quick Start

```bash
# Start infrastructure
just db-up
just migrate

# Run backend (in separate terminals)
just dev      # API server (:3001)
just devr     # Realtime server (:3002)

# Run frontend
cd frontend && bun install && bun run dev  # :5173
```

## Project Structure

```
crates/
  api/        # REST API (Axum)
  realtime/   # Socket.IO server
  core/       # Domain logic
  db/         # Database layer
  auth/       # Authentication
  protocol/   # Shared DTOs
frontend/     # SvelteKit app
```

## Development

```bash
cargo test --workspace     # Run tests
cargo fmt --all            # Format code
cargo clippy --workspace   # Lint
```

## Credits

- **Idea**: Inspired by [WorldGuesser](https://worldguesser.net) — a free, open-source GeoGuessr alternative
- **Locations**: Generated using [Vali](https://github.com/slashP/Vali) — a tool for creating GeoGuessr maps from Google Street View and OpenStreetMap data
