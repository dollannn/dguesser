# R2 CDN Setup Guide

This guide documents how to set up Cloudflare R2 as the storage backend for DGuesser's location pack files.

## Overview

DGuesser uses a **hybrid architecture** for location data:

- **PostgreSQL**: Users, games, sessions, and location reports (~1GB)
- **Cloudflare R2**: 100M+ location pack files (~10-50GB)

This achieves the **$5-10/month hosting target** by leveraging R2's cheap storage ($0.015/GB/mo) and free egress.

### Architecture Diagram

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   SvelteKit     │────▶│   Rust API       │────▶│  PostgreSQL     │
│   Frontend      │     │   (Axum)         │     │  (users, games) │
└─────────────────┘     └────────┬─────────┘     └─────────────────┘
                                 │
                                 │ HTTP Range Requests
                                 ▼
                        ┌──────────────────┐
                        │  Cloudflare R2   │
                        │  cdn.dguesser.lol│
                        │  (location packs)│
                        └──────────────────┘
```

**Key points:**
- The **backend** fetches location data from R2 (server-to-server)
- The **frontend** never directly accesses R2
- No CORS required for server-side access, but configured for future flexibility

---

## R2 Bucket Structure

```
dguesser-cdn/
└── v2026-01/                    # Version directory
    ├── manifest.json            # Global manifest with country stats
    └── countries/
        ├── US/
        │   ├── index.json       # Country-specific bucket index
        │   ├── US_B4_S0.pack    # Pack files (year bucket + scout bucket)
        │   ├── US_B5_S0.pack
        │   └── ...
        ├── FR/
        │   ├── index.json
        │   └── ...
        └── ... (126 countries)
```

### File Formats

**manifest.json** - Global index:
```json
{
  "schema_version": 1,
  "version": "v2026-01",
  "build_date": "2026-01-16T14:48:32Z",
  "countries": {
    "US": { "count": 1500000 },
    "FR": { "count": 800000 }
  }
}
```

**countries/{CC}/index.json** - Country index:
```json
{
  "country": "US",
  "version": "v2026-01",
  "record_size": 192,
  "buckets": {
    "B4_S0": { "count": 44, "object": "US_B4_S0.pack" },
    "B5_S0": { "count": 156, "object": "US_B5_S0.pack" }
  }
}
```

**.pack files** - Binary location records (192 bytes each):
- Fixed-size records for efficient HTTP Range requests
- Contains: lat, lng, pano_id, heading, capture_year, etc.

---

## Setup Instructions

### Step 1: Create R2 Bucket

1. Log in to [Cloudflare Dashboard](https://dash.cloudflare.com)
2. Navigate to **R2 Object Storage** in the sidebar
3. Click **Create bucket**
4. Name it (e.g., `dguesser-cdn`)
5. Select your preferred location (EU or US)

### Step 2: Enable Public Access (Custom Domain)

1. Go to **R2** > Select your bucket > **Settings**
2. Under **Public access**, click **Connect Domain**
3. Enter your custom domain: `cdn.dguesser.lol`
4. Cloudflare will automatically configure DNS and SSL

Your bucket is now accessible at `https://cdn.dguesser.lol/`

### Step 3: Configure CORS (Optional but Recommended)

Even though the backend doesn't require CORS, configure it for future flexibility:

1. Go to **R2** > Select your bucket > **Settings** > **CORS Policy**
2. Click **Add CORS policy** and enter:

```json
[
  {
    "AllowedOrigins": [
      "https://dguesser.lol",
      "https://www.dguesser.lol",
      "http://localhost:5173"
    ],
    "AllowedMethods": ["GET", "HEAD"],
    "AllowedHeaders": ["Range", "Content-Type"],
    "ExposeHeaders": ["Content-Length", "Content-Range", "ETag"],
    "MaxAgeSeconds": 86400
  }
]
```

### Step 4: Create R2 API Credentials

For uploading files, you need S3-compatible API credentials:

1. Go to **R2** > **Manage R2 API Tokens** (in sidebar)
2. Click **Create API token**
3. Configure:
   - **Token name**: `dguesser-upload`
   - **Permissions**: **Object Read & Write**
   - **Bucket scope**: Select your bucket (`dguesser-cdn`)
4. Click **Create API Token**
5. **Save these credentials** (shown only once):
   - `Access Key ID`
   - `Secret Access Key`
   - `Endpoint URL` (e.g., `https://<account-id>.r2.cloudflarestorage.com`)

---

## Uploading Pack Files

### Install rclone

**Arch Linux:**
```bash
sudo pacman -S rclone
```

**macOS:**
```bash
brew install rclone
```

**Ubuntu/Debian:**
```bash
sudo apt install rclone
```

### Configure rclone

Create `~/.config/rclone/rclone.conf`:

```ini
[dguesser-r2]
type = s3
provider = Cloudflare
access_key_id = YOUR_ACCESS_KEY_ID
secret_access_key = YOUR_SECRET_ACCESS_KEY
endpoint = https://YOUR_ACCOUNT_ID.r2.cloudflarestorage.com
acl = private
```

Replace:
- `YOUR_ACCESS_KEY_ID` with your R2 Access Key ID
- `YOUR_SECRET_ACCESS_KEY` with your R2 Secret Access Key
- `YOUR_ACCOUNT_ID` with your Cloudflare account ID (from the endpoint URL)

### Verify Connection

```bash
# List bucket contents (may show nothing if empty)
rclone ls dguesser-r2:dguesser-cdn

# Note: ListBuckets may fail with 403 - that's OK, bucket access still works
```

### Upload Pack Files

**Dry run first** (verify what will be uploaded):
```bash
rclone sync /path/to/packs/v2026-01 dguesser-r2:dguesser-cdn/v2026-01 --dry-run --progress
```

**Actual upload** (parallel transfers for speed):
```bash
rclone sync /path/to/packs/v2026-01 dguesser-r2:dguesser-cdn/v2026-01 --progress --transfers 16
```

**Verify upload:**
```bash
# Check files in R2
rclone ls dguesser-r2:dguesser-cdn/v2026-01 | head -20

# Test public access
curl -s "https://cdn.dguesser.lol/v2026-01/manifest.json" | head -10
```

---

## Environment Configuration

### Development (Local Files)

For local development without R2 access:

```bash
# .env
LOCATION_PROVIDER=r2
LOCATION_R2_URL=file:///path/to/local/packs
LOCATION_R2_VERSION=v2026-01
```

### Production (R2)

```bash
# .env
LOCATION_PROVIDER=r2
LOCATION_R2_URL=https://cdn.dguesser.lol
LOCATION_R2_VERSION=v2026-01
```

### Environment Variables Reference

| Variable | Description | Example |
|----------|-------------|---------|
| `LOCATION_PROVIDER` | Provider type: `postgres` or `r2` | `r2` |
| `LOCATION_R2_URL` | Base URL for R2 or local path | `https://cdn.dguesser.lol` |
| `LOCATION_R2_VERSION` | Dataset version directory | `v2026-01` |
| `LOCATION_MAX_DISABLED_CACHE` | Max disabled location hashes in memory | `200000` |

---

## Generating Pack Files

Pack files are generated from Vali data using the `pack_builder` binary:

```bash
# Build the pack builder
cargo build --release -p dguesser-locations --bin pack_builder

# Run it (see pack_builder --help for options)
./target/release/pack_builder \
    --input ~/vali-data/Vali \
    --output ~/dguesser-packs/packs \
    --version v2026-01
```

This will:
1. Read all Vali `.bin` files (protobuf format)
2. Convert to fixed-size binary pack format
3. Organize by country and year/scout buckets
4. Generate `manifest.json` and `index.json` files

---

## Monthly Updates

When Vali releases new data:

### 1. Generate New Pack Files

```bash
./target/release/pack_builder \
    --input ~/vali-data/Vali \
    --output ~/dguesser-packs/packs \
    --version v2026-02
```

### 2. Upload to R2

```bash
rclone sync ~/dguesser-packs/packs/v2026-02 dguesser-r2:dguesser-cdn/v2026-02 --progress --transfers 16
```

### 3. Update Environment

```bash
# .env
LOCATION_R2_VERSION=v2026-02
```

### 4. Restart Services

```bash
# Restart API and Realtime servers to pick up new version
just dev
```

Old versions can be kept in R2 for rollback, or deleted to save storage:

```bash
# Delete old version
rclone purge dguesser-r2:dguesser-cdn/v2026-01
```

---

## Troubleshooting

### "Access Denied" when listing buckets

This is normal - R2 tokens may not have `ListBuckets` permission. Direct bucket access still works:
```bash
rclone ls dguesser-r2:dguesser-cdn/v2026-01
```

### 403 Forbidden on public URL

1. Verify custom domain is connected in R2 settings
2. Check DNS propagation: `dig cdn.dguesser.lol`
3. Verify the file path is correct (case-sensitive)

### Slow uploads

Increase parallel transfers:
```bash
rclone sync ... --transfers 32 --checkers 16
```

### CORS errors in browser

If you add browser-side R2 access later:
1. Check CORS policy includes your origin
2. Verify `AllowedMethods` includes the HTTP method used
3. Check browser DevTools Network tab for the actual error

---

## Cost Breakdown

| Component | Monthly Cost |
|-----------|--------------|
| R2 Storage (50GB) | ~$0.75 |
| R2 Egress | **FREE** |
| R2 Class A ops (writes) | ~$0.50 (upload only) |
| R2 Class B ops (reads) | ~$0.36/million |
| **Total R2** | **~$1-2/month** |

Combined with Railway PostgreSQL (~$5/month), total hosting: **~$6-7/month**

---

## Security Notes

1. **Never commit credentials** - Keep R2 keys out of git
2. **Credentials are for uploads only** - Reads use public URL, no auth needed
3. **rclone config contains secrets** - `~/.config/rclone/rclone.conf` should be protected
4. **Rotate keys periodically** - Create new tokens and delete old ones in Cloudflare dashboard

---

## Quick Reference

```bash
# Upload new version
rclone sync /path/to/packs/vYYYY-MM dguesser-r2:dguesser-cdn/vYYYY-MM --progress --transfers 16

# Verify upload
curl -s "https://cdn.dguesser.lol/vYYYY-MM/manifest.json"

# Check storage usage
rclone size dguesser-r2:dguesser-cdn

# Delete old version
rclone purge dguesser-r2:dguesser-cdn/vOLD-VERSION
```
