# DGuesser Development Commands

# Default recipe - show available commands
default:
    @just --list

# ============================================================================
# DEVELOPMENT
# ============================================================================

# Start all dev services: ensures Docker infra + launches API, Realtime, Web in tmux
dev:
    #!/usr/bin/env bash
    set -euo pipefail

    SESSION="dguesser"
    ROOT="{{justfile_directory()}}"
    LOG_DIR="$ROOT/.logs"

    # Preflight checks
    command -v tmux >/dev/null || { echo "[dguesser] tmux is required but not installed"; exit 1; }
    command -v docker >/dev/null || { echo "[dguesser] docker is required but not installed"; exit 1; }

    # Reattach if already running
    if tmux has-session -t "$SESSION" 2>/dev/null; then
        tmux attach -t "$SESSION"; exit 0
    fi

    mkdir -p "$LOG_DIR"

    # Ensure infrastructure is running (blocks until all health checks pass)
    echo "[dguesser] Starting infrastructure..."
    docker compose -f "$ROOT/docker-compose.yml" up -d --wait

    # Window 1: API server
    tmux new-session -d -s "$SESSION" -n api -c "$ROOT"
    tmux send-keys -t "$SESSION:api" "watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-api" Enter

    # Window 2: Realtime server
    tmux new-window -t "$SESSION" -n realtime -c "$ROOT"
    tmux send-keys -t "$SESSION:realtime" "watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-realtime" Enter

    # Window 3: Web (SvelteKit frontend)
    tmux new-window -t "$SESSION" -n web -c "$ROOT/frontend"
    : > "$LOG_DIR/web.log"
    tmux pipe-pane -t "$SESSION:web" -o "cat >> '$LOG_DIR/web.log'"
    tmux send-keys -t "$SESSION:web" "bun run dev" Enter

    # Window 4: empty shell for ad-hoc commands
    tmux new-window -t "$SESSION" -n shell -c "$ROOT"

    # Focus and attach
    tmux select-window -t "$SESSION:api"
    echo "[dguesser] api       http://localhost:3001"
    echo "[dguesser] realtime  http://localhost:3002"
    echo "[dguesser] web       http://localhost:5173"
    echo "[dguesser] logs      .logs/web.log"
    tmux attach -t "$SESSION"

# Kill the tmux dev session and stop infrastructure
kill:
    #!/usr/bin/env bash
    if tmux has-session -t dguesser 2>/dev/null; then
        tmux kill-session -t dguesser
        echo "Stopped 'dguesser' session."
    else
        echo "No session running."
    fi
    docker compose down
    echo "Stopped infrastructure containers."

# Restart a specific service window (api, realtime, web)
restart service:
    #!/usr/bin/env bash
    set -euo pipefail
    SESSION="dguesser"
    if ! tmux has-session -t "$SESSION" 2>/dev/null; then
        echo "No session running. Use 'just dev' to start."
        exit 1
    fi
    ROOT="{{justfile_directory()}}"
    case "{{service}}" in
        api)       tmux respawn-pane -k -t "$SESSION:api" -c "$ROOT" "watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-api" ;;
        realtime)  tmux respawn-pane -k -t "$SESSION:realtime" -c "$ROOT" "watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-realtime" ;;
        web)       tmux respawn-pane -k -t "$SESSION:web" -c "$ROOT/frontend" "bun run dev" ;;
        *)         echo "Unknown service: {{service}}. Use: api, realtime, web"; exit 1 ;;
    esac

# Attach to tmux dev session (optionally jump to a specific window)
logs service="":
    #!/usr/bin/env bash
    SESSION="dguesser"
    if ! tmux has-session -t "$SESSION" 2>/dev/null; then
        echo "No session running. Use 'just dev' to start."
        exit 1
    fi
    if [ -n "{{service}}" ]; then
        tmux select-window -t "$SESSION:{{service}}"
    fi
    tmux attach -t "$SESSION"

# ============================================================================
# INDIVIDUAL SERVICES (run outside tmux)
# ============================================================================

# Run API server with hot reload
dev-api:
    watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-api

# Run realtime server with hot reload
dev-realtime:
    watchexec -r -e rs,toml -w crates -- cargo run -p dguesser-realtime

# Run frontend dev server
dev-web:
    cd frontend && bun run dev

# ============================================================================
# INFRASTRUCTURE
# ============================================================================

# Start infrastructure (PostgreSQL + Redis via Docker Compose)
infra:
    docker compose up -d --wait

# Stop infrastructure
infra-down:
    docker compose down

# Run database migrations
migrate:
    sqlx migrate run

# ============================================================================
# TESTING & CODE QUALITY
# ============================================================================

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
