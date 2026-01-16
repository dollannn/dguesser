# SEO Optimization Implementation Plan

> Comprehensive SEO overhaul for DGuesser frontend

## Project Summary

**Goal**: Transform DGuesser from basic SEO to fully optimized for search engines and social sharing.

**Production Domain**: `https://dguesser.lol`

## Current State

| What's There | What's Missing |
|--------------|----------------|
| Basic meta description on homepage | Open Graph tags (social sharing) |
| Page titles on most routes | Twitter Card tags |
| robots.txt (allows all) | Per-page meta descriptions |
| Favicons | sitemap.xml |
| SSR enabled by default | Structured data (JSON-LD) |
| | Canonical URLs |
| | Admin pages not blocked from crawlers |
| | Server-side data loading for SEO |

**Notable Issues**:
- Title inconsistency: `/game/[id]` uses lowercase "dguesser"
- All data loading is client-side (`$effect()`) - crawlers see empty content
- No hooks.server.ts for request handling

## Implementation Phases

| Phase | Description | Priority | Files |
|-------|-------------|----------|-------|
| [01 - Core Infrastructure](./01-core-infrastructure.md) | SEO component + server API helper | High | 2 new |
| [02 - SSR Data Loading](./02-ssr-data-loading.md) | Server-side data for crawlers | High | 5 new |
| [03 - Dynamic Sitemap](./03-dynamic-sitemap.md) | Auto-generated sitemap.xml | High | 1 new |
| [04 - Apply SEO to Pages](./04-apply-seo-all-pages.md) | Add SEO component everywhere | High | ~15 modified |
| [05 - Structured Data](./05-structured-data.md) | JSON-LD schemas | Medium | 2 modified |
| [06 - Final Touches](./06-final-touches.md) | robots.txt, prerender, noindex | Low | 4 modified |

## Configuration

```
Site URL:     https://dguesser.lol
OG Image:     /favicon.svg (existing logo)
Sitemap:      Public maps + static pages (no user profiles)
```

## Files Overview

### New Files (8)
```
frontend/src/lib/components/SEO.svelte      # Reusable SEO component
frontend/src/lib/server/api.ts              # Server-side API helper
frontend/src/routes/sitemap.xml/+server.ts  # Dynamic sitemap
frontend/src/routes/maps/+page.server.ts    # SSR for maps list
frontend/src/routes/maps/[id]/+page.server.ts
frontend/src/routes/leaderboard/+page.server.ts
frontend/src/routes/u/[username]/+page.server.ts
```

### Modified Files (~15)
```
frontend/src/app.d.ts                       # PageData types
frontend/src/routes/+layout.svelte          # Organization JSON-LD
frontend/src/routes/+page.svelte            # Homepage SEO
frontend/src/routes/play/+page.svelte
frontend/src/routes/maps/+page.svelte
frontend/src/routes/maps/[id]/+page.svelte
frontend/src/routes/maps/[id]/edit/+page.svelte
frontend/src/routes/maps/new/+page.svelte
frontend/src/routes/leaderboard/+page.svelte
frontend/src/routes/u/[username]/+page.svelte
frontend/src/routes/game/[id]/+page.svelte  # Fix title + noindex
frontend/src/routes/auth/+page.svelte       # noindex
frontend/src/routes/privacy/+page.svelte    # prerender
frontend/src/routes/terms/+page.svelte      # prerender
frontend/static/robots.txt
```

## Success Criteria

- [ ] All public pages have unique titles and descriptions
- [ ] Social sharing shows rich previews (OG + Twitter)
- [ ] sitemap.xml includes all public maps
- [ ] Google can crawl and index map content
- [ ] Admin/auth pages excluded from search results
- [ ] Lighthouse SEO score > 90

## Dependencies

- No external packages needed
- Uses existing SvelteKit features
- Requires `$env/static/public` for site URL configuration

## Environment Variables

Add to `.env`:
```bash
PUBLIC_SITE_URL=https://dguesser.lol
```

## Estimated Effort

- **Total**: ~4-6 hours
- Phase 1: 1 hour
- Phase 2: 1.5 hours
- Phase 3: 30 min
- Phase 4: 1.5 hours
- Phase 5: 30 min
- Phase 6: 30 min
