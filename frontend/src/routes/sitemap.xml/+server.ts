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
  { path: '/history', priority: '0.6', changefreq: 'daily' },
  { path: '/privacy', priority: '0.3', changefreq: 'monthly' },
  { path: '/terms', priority: '0.3', changefreq: 'monthly' }
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
        createUrlEntry(`/maps/${map.id}`, '0.7', 'weekly', map.updated_at)
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
      'Cache-Control': 'max-age=3600' // Cache for 1 hour
    }
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
