# DGuesser Development Commands

# Default recipe - show available commands
default:
    @just --list

# Run API server with hot reload
dev:
    watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-api

# Run realtime server with hot reload
devr:
    watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-realtime

# Run both API and realtime servers with hot reload
deva:
    #!/usr/bin/env bash
    set -e
    trap 'kill $(jobs -p) 2>/dev/null' EXIT
    just dev &
    just devr &
    wait

# Start database containers (PostgreSQL + Redis)
db-up:
    docker-compose up -d postgres redis

# Stop database containers
db-down:
    docker-compose down

# Run database migrations
migrate:
    sqlx migrate run

# Run all tests
test:
    cargo test --workspace

# Run clippy linter (warnings as errors)
lint:
    cargo clippy --workspace -- -D warnings

# Format all code
fmt:
    cargo fmt --all

# Check formatting without changes
fmt-check:
    cargo fmt --all -- --check

# Build all crates
build:
    cargo build --workspace

# Build release
build-release:
    cargo build --workspace --release
