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

### Key Libraries

| Purpose | Library | Notes |
|---------|---------|-------|
| ID Generation | `nanoid` | Prefixed public IDs (`usr_`, `gam_`, etc.) |
| Session Tokens | `rand_chacha` | ChaCha20 CSPRNG for 256-bit tokens |
| API Documentation | `utoipa` | OpenAPI/Swagger at `/docs` |
| Base64 Encoding | `base64-url` | URL-safe encoding for session tokens |

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

### 4. Prefixed Nanoid IDs (Not UUIDs)

**Decision:** Use nanoid with prefixes for all entity IDs (`usr_`, `gam_`, `ses_`, etc.)

**Rationale:**
- Human-readable prefixes identify entity type at a glance
- URL-safe characters (no encoding needed in routes)
- Smaller storage footprint than UUIDs (16 chars vs 36)
- ~71 bits entropy for entities, 256 bits for sessions
- Consistent ID format across entire stack

| Entity | Format | Example |
|--------|--------|---------|
| User | `usr_xxxxxxxxxxxx` | `usr_V1StGXR8_Z5j` |
| Game | `gam_xxxxxxxxxxxx` | `gam_FybH2oF9Xaw8` |
| Session | `ses_xxx...(43)` | `ses_Uakgb_J5m9g-...` |
| Round | `rnd_xxxxxxxxxxxx` | `rnd_Q3kT7bN2mPxW` |
| Guess | `gss_xxxxxxxxxxxx` | `gss_L9vR4cD8sHjK` |
| OAuth | `oau_xxxxxxxxxxxx` | `oau_M2nP6fG1tYqZ` |

### 5. ChaCha20 for Session Tokens

**Decision:** Use `rand_chacha` (ChaCha20 RNG) for cryptographically secure session tokens.

**Rationale:**
- 256 bits of entropy (extremely secure)
- ChaCha20 is a well-audited CSPRNG
- Thread-safe implementation via `Mutex<ChaCha20Rng>`
- Prefixed with `ses_` for consistency

### 6. OpenAPI Documentation with utoipa

**Decision:** Generate OpenAPI docs at compile-time using `utoipa`.

**Rationale:**
- Type-safe documentation derived from code
- Swagger UI available at `/docs`
- No runtime overhead
- Self-documenting API endpoints

### 7. Redis for Hot State, Postgres for Truth

**Decision:** 
- Redis: Active game state, sessions, pub/sub, presence
- Postgres: All durable data (users, completed games, history)

**Rationale:**
- Fast real-time operations without DB pressure
- Horizontal scaling via Redis coordination
- Clear separation of concerns

### 8. Railway for Hosting

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
