# dguesser Implementation Plan - Overview

## Project Vision

A GeoGuessr clone featuring real-time multiplayer gameplay, guest support, and OAuth authentication.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | SvelteKit |
| API Backend | Rust + Axum + SQLx |
| Realtime Backend | Rust + Axum + SQLx + SocketIOxide |
| Database | PostgreSQL |
| Cache/Pub-Sub | Redis |
| Hosting | Railway (containers) |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      SvelteKit Frontend                      │
│                   (SSR + Client-side SPA)                    │
└──────────────┬────────────────────────┬─────────────────────┘
               │ REST API               │ Socket.IO
               ▼                        ▼
┌──────────────────────────┐  ┌──────────────────────────────┐
│       API Crate          │  │      Realtime Crate          │
│  - Auth/OAuth            │  │  - Game sessions             │
│  - User management       │  │  - Live gameplay             │
│  - Maps/locations        │  │  - Room management           │
│  - Leaderboards          │  │  - Chat (future)             │
│  - Game history          │  │                              │
└──────────┬───────────────┘  └──────────┬───────────────────┘
           │                             │
           │    ┌─────────────────┐      │
           │    │  Shared Crates  │      │
           │    │  - core         │      │
           │    │  - db           │      │
           │    │  - auth         │      │
           │    │  - protocol     │      │
           │    └─────────────────┘      │
           │             │               │
           ▼             ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                       PostgreSQL                             │
│  Users, Games, Rounds, Guesses, OAuth, Sessions             │
└─────────────────────────────────────────────────────────────┘
                         │
┌─────────────────────────────────────────────────────────────┐
│                         Redis                                │
│  Sessions, Game State, Pub/Sub, Rate Limiting               │
└─────────────────────────────────────────────────────────────┘
```

## Rust Workspace Structure

```
dguesser/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── core/               # Domain logic, game rules, scoring
│   ├── db/                 # SQLx queries, row types, migrations
│   ├── auth/               # OAuth, session management
│   ├── protocol/           # Shared DTOs, socket events
│   ├── api/                # REST API binary
│   └── realtime/           # Socket.IO binary
├── migrations/             # Shared SQL migrations
├── frontend/               # SvelteKit app
└── docs/
    └── impl-plan/          # This documentation
```

## Implementation Phases

| Phase | Name | Priority | Est. Duration |
|-------|------|----------|---------------|
| 1 | [Project Foundation](./01-project-foundation.md) | P0 | 2-3 days |
| 2 | [Database & Core Domain](./02-database-core.md) | P0 | 3-4 days |
| 3 | [Authentication System](./03-authentication.md) | P0 | 3-4 days |
| 4 | [API Crate](./04-api-crate.md) | P0 | 4-5 days |
| 5 | [Realtime Crate](./05-realtime-crate.md) | P0 | 4-5 days |
| 6 | [Frontend Foundation](./06-frontend-foundation.md) | P0 | 3-4 days |
| 7 | [Game Experience](./07-game-experience.md) | P1 | 5-7 days |
| 8 | [Polish & Production](./08-polish-production.md) | P2 | 3-5 days |

## Key Architectural Decisions

### 1. Server-Side Sessions (Not JWTs in Browser)

**Decision:** Use `HttpOnly` cookies with server-side session storage.

**Rationale:**
- Works seamlessly for both HTTP and WebSocket (cookies auto-sent)
- No XSS token theft risk
- Easy session revocation
- Same auth story for API and Realtime

### 2. Guests as Real Users

**Decision:** Create actual `users` rows for guests (`kind = 'guest'`).

**Rationale:**
- Every guess/game can safely FK to `user_id`
- Migration to authenticated is a simple update/link
- Consistent query patterns throughout codebase

### 3. Realtime as Gameplay Authority

**Decision:** All gameplay commands flow through the Realtime crate.

**Rationale:**
- Single source of truth for game state
- Serialized command processing (actor-per-game model)
- Cheat-resistant architecture
- API handles everything else (auth, history, profiles)

### 4. Redis for Hot State, Postgres for Truth

**Decision:** 
- Redis: Active game state, sessions, pub/sub, presence
- Postgres: All durable data (users, completed games, history)

**Rationale:**
- Fast real-time operations without DB pressure
- Horizontal scaling via Redis coordination
- Clear separation of concerns

### 5. Railway for Hosting

**Decision:** Deploy all services as containers on Railway.

**Rationale:**
- Native Docker/Nixpacks support
- Built-in Postgres and Redis services with private networking
- Automatic HTTPS and custom domains
- Easy environment variable management
- Per-service scaling and resource allocation
- Integrated logging and metrics

## Dependencies Between Phases

```
Phase 1 (Foundation)
    │
    ▼
Phase 2 (Database & Core) ──────────┐
    │                               │
    ▼                               ▼
Phase 3 (Authentication)      Phase 6 (Frontend Foundation)
    │                               │
    ├───────────────────────────────┤
    │                               │
    ▼                               │
Phase 4 (API) ◄─────────────────────┘
    │
    ▼
Phase 5 (Realtime)
    │
    ▼
Phase 7 (Game Experience)
    │
    ▼
Phase 8 (Polish & Production)
```

## Success Criteria

### MVP (Phases 1-6)
- [ ] Guest users can play solo games
- [ ] OAuth login with Google/Microsoft works
- [ ] Game sessions persist to database
- [ ] Basic UI renders correctly

### Beta (Phase 7)
- [ ] Real-time multiplayer works
- [ ] Multiple simultaneous games supported
- [ ] Reconnection handling works
- [ ] Leaderboards functional

### Production (Phase 8)
- [ ] Rate limiting in place
- [ ] Monitoring/observability configured
- [ ] Error handling polished
- [ ] Performance optimized
