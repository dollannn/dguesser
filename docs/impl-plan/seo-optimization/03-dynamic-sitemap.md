# Phase 3: Dynamic Sitemap

> Auto-generate sitemap.xml with all public content

## Objectives

Create a dynamic sitemap endpoint that includes:
- Static pages (homepage, play, leaderboard, privacy, terms)
- All public maps from the database

**Not included** (per requirements): User profiles

---

## Task 3.1: Create Sitemap Endpoint

**File**: `frontend/src/routes/sitemap.xml/+server.ts`

### Implementation

```typescript
import { serverApi } from '$lib/server/api';
import type { RequestHandler } from './$types';

const SITE_URL = 'https://dguesser.lol';

interface MapSummary {
  id: string;
  slug: string;
  name: string;
  visibility: string;
  updated_at?: string;
}

interface ListMapsResponse {
  maps: MapSummary[];
}

// Static pages with their priorities and change frequencies
const STATIC_PAGES = [
  { path: '/', priority: '1.0', changefreq: 'daily' },
  { path: '/play', priority: '0.9', changefreq: 'daily' },
  { path: '/leaderboard', priority: '0.8', changefreq: 'hourly' },
  { path: '/maps', priority: '0.8', changefreq: 'daily' },
  { path: '/privacy', priority: '0.3', changefreq: 'monthly' },
  { path: '/terms', priority: '0.3', changefreq: 'monthly' },
];

export const GET: RequestHandler = async () => {
  const urls: string[] = [];

  // Add static pages
  for (const page of STATIC_PAGES) {
    urls.push(createUrlEntry(page.path, page.priority, page.changefreq));
  }

  // Fetch public maps
  try {
    const response = await serverApi.get<ListMapsResponse>('/maps');
    const publicMaps = response.maps.filter((m) => m.visibility === 'public');

    for (const map of publicMaps) {
      urls.push(
        createUrlEntry(
          `/maps/${map.id}`,
          '0.7',
          'weekly',
          map.updated_at
        )
      );
    }
  } catch (e) {
    console.error('Failed to fetch maps for sitemap:', e);
    // Continue without maps - static pages are still valid
  }

  const sitemap = createSitemap(urls);

  return new Response(sitemap, {
    headers: {
      'Content-Type': 'application/xml',
      'Cache-Control': 'max-age=3600', // Cache for 1 hour
    },
  });
};

function createUrlEntry(
  path: string,
  priority: string,
  changefreq: string,
  lastmod?: string
): string {
  const loc = `${SITE_URL}${path}`;
  let entry = `  <url>\n    <loc>${escapeXml(loc)}</loc>\n`;

  if (lastmod) {
    // Format as YYYY-MM-DD
    const date = new Date(lastmod).toISOString().split('T')[0];
    entry += `    <lastmod>${date}</lastmod>\n`;
  }

  entry += `    <changefreq>${changefreq}</changefreq>\n`;
  entry += `    <priority>${priority}</priority>\n`;
  entry += `  </url>`;

  return entry;
}

function createSitemap(urls: string[]): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls.join('\n')}
</urlset>`;
}

function escapeXml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}
```

---

## Task 3.2: Prerender Sitemap (Optional)

If you want the sitemap to be generated at build time instead of on each request:

**File**: `frontend/src/routes/sitemap.xml/+server.ts`

Add at the top:
```typescript
export const prerender = true;
```

**Note**: This means the sitemap won't update until the next deployment. For dynamic content, keep it as a server route (no prerender).

---

## Example Output

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://dguesser.lol/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/play</loc>
    <changefreq>daily</changefreq>
    <priority>0.9</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/leaderboard</loc>
    <changefreq>hourly</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/maps</loc>
    <changefreq>daily</changefreq>
    <priority>0.8</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/maps/abc123</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.7</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/maps/def456</loc>
    <lastmod>2024-01-10</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.7</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/privacy</loc>
    <changefreq>monthly</changefreq>
    <priority>0.3</priority>
  </url>
  <url>
    <loc>https://dguesser.lol/terms</loc>
    <changefreq>monthly</changefreq>
    <priority>0.3</priority>
  </url>
</urlset>
```

---

## Priority Guidelines

| Priority | Usage |
|----------|-------|
| 1.0 | Homepage |
| 0.9 | High-value pages (play) |
| 0.8 | Important discovery pages (maps, leaderboard) |
| 0.7 | Individual content pages (map details) |
| 0.5 | Default |
| 0.3 | Legal/utility pages |

## Change Frequency Guidelines

| Frequency | Usage |
|-----------|-------|
| hourly | Leaderboard (scores change frequently) |
| daily | Homepage, maps list, play |
| weekly | Individual maps |
| monthly | Legal pages |

---

## Verification Checklist

- [ ] `/sitemap.xml` returns valid XML
- [ ] Static pages are included
- [ ] Public maps are included
- [ ] Private/unlisted maps are NOT included
- [ ] XML is properly escaped
- [ ] Validate with: https://www.xml-sitemaps.com/validate-xml-sitemap.html

## Testing

```bash
# Fetch sitemap
curl -s https://dguesser.lol/sitemap.xml

# Validate XML structure
curl -s https://dguesser.lol/sitemap.xml | xmllint --noout -

# Count URLs
curl -s https://dguesser.lol/sitemap.xml | grep -c '<url>'
```

## Files Changed

| File | Action |
|------|--------|
| `routes/sitemap.xml/+server.ts` | Create |
