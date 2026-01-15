# Handoff Prompt: DGuesser Vali Data Ingestion Architecture

## Context for GPT-5.2 / External Architect

You are designing the cheapest possible data architecture for DGuesser, a GeoGuessr clone. The user has 41GB of Vali location data and needs to serve 100M+ Street View locations for gameplay.

---

## Project Overview

**DGuesser** is a geography guessing game:
- **Backend**: Rust workspace (Axum API, Socket.IO realtime)
- **Frontend**: SvelteKit 5, TypeScript, Tailwind CSS
- **Current DB**: PostgreSQL 17 + Redis via Docker Compose
- **Production Target**: Railway (but open to alternatives for cost)

---

## Current Database Schema

### Locations Table
```sql
CREATE TABLE locations (
    id VARCHAR(16) PRIMARY KEY,              -- loc_XXXXXXXXXXXX
    panorama_id VARCHAR(100) NOT NULL UNIQUE, -- Google Street View pano ID
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    country_code CHAR(2),                    -- ISO 3166-1 alpha-2
    subdivision_code VARCHAR(10),            -- ISO 3166-2
    capture_date DATE,
    provider VARCHAR(50) DEFAULT 'google_streetview',
    active BOOLEAN DEFAULT TRUE,
    validation_status VARCHAR(20) DEFAULT 'ok',
    random_key DOUBLE PRECISION DEFAULT random(), -- For O(1) random selection
    
    -- Vali metadata
    source VARCHAR(32) DEFAULT 'vali',
    surface VARCHAR(64),                     -- road surface (asphalt, gravel, etc.)
    arrow_count INTEGER,
    is_scout BOOLEAN DEFAULT FALSE,          -- trekker/gen3 coverage
    buildings_100 INTEGER,                   -- OSM building density
    roads_100 INTEGER,
    elevation INTEGER,
    heading DOUBLE PRECISION,
    
    -- Failure/review tracking
    failure_count INTEGER DEFAULT 0,
    review_status VARCHAR(32) DEFAULT 'approved',
    
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Key indexes
CREATE INDEX idx_locations_random ON locations(random_key) WHERE active = TRUE;
CREATE INDEX idx_locations_country ON locations(country_code) WHERE active = TRUE;
```

### Maps Table (Rules-Based)
```sql
CREATE TABLE maps (
    id VARCHAR(16) PRIMARY KEY,
    slug VARCHAR(50) UNIQUE,
    name VARCHAR(100),
    -- Rules define filtering, NOT explicit membership
    rules JSONB DEFAULT '{}',  -- {"countries": ["US", "CA"], "min_year": 2015}
    is_default BOOLEAN DEFAULT FALSE,
    active BOOLEAN DEFAULT TRUE
);

-- Example maps:
-- 'world': {} (no filter = all locations)
-- 'usa': {"countries": ["US"]}
-- 'europe': {"countries": ["FR", "DE", "IT", ...]}
```

### User Decision: Rules-Based Maps
The user chose **rules-based maps** over explicit membership. This means:
- NO `map_locations` junction table for system maps
- "World" map = implicit (all active locations)
- Maps filter by `country_code`, `subdivision_code`, `min_year`, etc.
- **Saves ~50% storage** vs explicit membership

---

## Vali Data Structure

**Location**: `~/vali-data/Vali/` (41GB total)

**Organization**:
```
Vali/
├── AD/                          # Andorra
│   ├── AD+AD-02.bin            # Parish 02
│   ├── AD+AD-03.bin
│   └── downloads.json
├── US/                          # United States
│   ├── US+US-AK.bin            # Alaska
│   ├── US+US-AL.bin            # Alabama (245MB)
│   ├── US+US-CA.bin            # California (223MB)
│   └── ...
├── FR/                          # France (2.3GB total)
│   └── ...
└── ... (126 countries total)
```

**Stats**:
- 2,243 `.bin` files
- 41GB total
- Largest file: `GB+GB-ENG.bin` (824MB - England)
- Estimated 100M+ total locations

**File Format**: Protobuf-net serialized C# `Location[]` arrays

From Vali source (`slashP/Vali` repo):
```csharp
// src/Vali.Core/Location.cs
public class Location {
    public string PanoId { get; set; }
    public double Lat { get; set; }
    public double Lng { get; set; }
    public string CountryCode { get; set; }
    public string SubdivisionCode { get; set; }
    public DateTime? CaptureDate { get; set; }
    public string Surface { get; set; }
    public int? ArrowCount { get; set; }
    public bool IsScout { get; set; }
    public int? Buildings100 { get; set; }
    public int? Roads100 { get; set; }
    public int? Elevation { get; set; }
    public double? Heading { get; set; }
}
```

Files can be decoded with: `ProtoBuf.Serializer.Deserialize<Location[]>(stream)`

---

## Core Gameplay Query

The fundamental operation is:
```
"Give me N random locations matching these filters"
```

Filters include:
- `map_id` → resolves to country list from `maps.rules`
- `min_year` / `max_year` (capture date)
- `outdoor_only` (exclude is_scout=true)
- `exclude_ids` (locations already used this game)

Current Rust implementation uses "seek-then-wrap" on `random_key`:
```sql
SELECT id, panorama_id, lat, lng, country_code
FROM locations
WHERE country_code = ANY($countries)
  AND active = TRUE
  AND random_key >= $random_value
ORDER BY random_key
LIMIT 1
```

---

## User Goal: CHEAPEST Possible Architecture

Target: **$5-10/month total hosting cost**

### Cost Reference Points
| Service | Pricing |
|---------|---------|
| Cloudflare R2 | $0.015/GB/mo storage, FREE egress |
| Railway PostgreSQL | $0.25/GB/mo storage + compute |
| Neon Free Tier | 0.5GB free, $0.16/GB after |
| Supabase Pro | $25/mo + $0.125/GB over 8GB |

### Hybrid CDN Approach (Proposed)
- **R2**: Store 41GB location data → ~$0.60/mo
- **PostgreSQL** (Railway/Neon): Only users, games, overrides → <1GB → ~$0-5/mo
- **Total**: ~$5-10/mo vs $25-35/mo for full PostgreSQL

---

## Design Questions to Answer

### 1. Location Pack Format for R2
- Keep protobuf? Convert to Parquet? Custom binary? JSON lines?
- How to enable efficient random selection by country/filters?
- Should files be pre-shuffled for randomness?

### 2. R2 File Organization
- One file per country? Per subdivision?
- Need a manifest/index file?
- Versioning strategy for monthly updates?

### 3. Random Selection Architecture
```
Client Request: "5 random US locations, year >= 2018"
                          ↓
                    [??? HOW ???]
                          ↓
Response: [{pano_id, lat, lng, country}, ...]
```

Options:
- **Edge Worker** (Cloudflare Worker) fetches from R2, samples, returns?
- **Backend** fetches file, samples, returns?
- **Client** fetches index, picks random offsets, fetches locations?

### 4. PostgreSQL Scope (What MUST Be in DB)
Definitely:
- `users`, `sessions` (auth)
- `games`, `rounds`, `guesses` (gameplay history)
- `location_reports` (user-reported broken panos)

Maybe:
- `disabled_locations` (pano_ids marked as broken)
- `maps` (custom user maps? or just config?)

### 5. Ingestion Pipeline
```
Vali .bin files → [CONVERTER] → R2-optimized format → Upload to R2
```
- Rust CLI tool?
- What transformations?
- How to handle the protobuf-net format?

### 6. Monthly Update Strategy
- Vali updates monthly
- Just re-upload changed country files?
- How to handle removed/deprecated locations?

### 7. Failure Tracking Without Full DB
- When a pano fails client-side, how to track it?
- Bloom filter on R2? Separate small DB table?
- How does this affect random selection (avoiding bad panos)?

---

## Constraints

1. **Must work offline for dev** - Can't require R2 for local development
2. **Rust backend** - Solution should integrate with existing Rust workspace
3. **Low latency** - Random selection should be <100ms
4. **Monthly updates** - Must handle Vali's monthly releases
5. **Failure resilience** - Bad panos should be trackable and avoidable

---

## Deliverables Requested

1. **Architecture diagram** showing data flow
2. **File format specification** for R2-stored location packs
3. **Random selection algorithm** that works with CDN-stored data
4. **Rust implementation sketch** for:
   - Vali → R2 converter
   - Location selection service
5. **Cost breakdown** confirming $5-10/mo target
6. **Migration path** from current PostgreSQL-only design

---

## Example Location Data (for reference)

From seed file:
```json
{
  "lat": 48.858370,
  "lng": 2.294481,
  "pano_id": "CAoSK0FGMVFpcE5LSTd3dXdRLTdJcjdEb3JRM2lOd1VQc2VFd0N4c05ReExBdnM.",
  "country_code": "FR"
}
```

From Vali .bin (decoded):
- Same fields plus: surface, arrow_count, is_scout, buildings_100, roads_100, elevation, heading, capture_date, subdivision_code

---

## End Goal

A complete architecture document that enables:
1. Converting 41GB Vali data to R2-optimized format
2. Serving random locations for gameplay at <100ms latency
3. Tracking failed/broken locations
4. Monthly updates with minimal effort
5. Total hosting cost of $5-10/month
