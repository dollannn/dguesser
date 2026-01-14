.PHONY: dev dev-realtime db-up db-down migrate test lint fmt

dev:
	cargo watch -x 'run -p dguesser-api'

dev-realtime:
	cargo watch -x 'run -p dguesser-realtime'

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
