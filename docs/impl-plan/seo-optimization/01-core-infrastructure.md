# Phase 1: Core Infrastructure

> Create the foundational components for SEO across the app

## Objectives

1. Create a reusable SEO component for consistent meta tags
2. Create a server-side API helper for SSR data fetching

---

## Task 1.1: Create SEO Component

**File**: `frontend/src/lib/components/SEO.svelte`

### Props Interface

```typescript
interface SEOProps {
  title?: string;           // Page title (appended with " - DGuesser")
  description?: string;     // Meta description (max 160 chars)
  canonical?: string;       // Canonical URL path (e.g., "/maps")
  ogImage?: string;         // Open Graph image URL
  ogType?: 'website' | 'article' | 'profile';
  noindex?: boolean;        // Add noindex meta tag
  jsonLd?: object;          // JSON-LD structured data
}
```

### Implementation

```svelte
<script lang="ts">
  import { page } from '$app/stores';

  interface Props {
    title?: string;
    description?: string;
    canonical?: string;
    ogImage?: string;
    ogType?: 'website' | 'article' | 'profile';
    noindex?: boolean;
    jsonLd?: object;
  }

  const SITE_NAME = 'DGuesser';
  const SITE_URL = 'https://dguesser.lol';
  const DEFAULT_DESCRIPTION = 'Test your geography knowledge by guessing locations around the world. Play solo or compete with friends!';
  const DEFAULT_OG_IMAGE = '/favicon.svg';

  let {
    title,
    description = DEFAULT_DESCRIPTION,
    canonical,
    ogImage = DEFAULT_OG_IMAGE,
    ogType = 'website',
    noindex = false,
    jsonLd,
  }: Props = $props();

  // Compute full title
  let fullTitle = $derived(title ? `${title} - ${SITE_NAME}` : `${SITE_NAME} - Geography Guessing Game`);

  // Compute canonical URL
  let canonicalUrl = $derived(canonical ? `${SITE_URL}${canonical}` : `${SITE_URL}${$page.url.pathname}`);

  // Compute OG image URL
  let ogImageUrl = $derived(ogImage.startsWith('http') ? ogImage : `${SITE_URL}${ogImage}`);
</script>

<svelte:head>
  <!-- Primary Meta Tags -->
  <title>{fullTitle}</title>
  <meta name="description" content={description} />
  <link rel="canonical" href={canonicalUrl} />

  {#if noindex}
    <meta name="robots" content="noindex, nofollow" />
  {/if}

  <!-- Open Graph / Facebook -->
  <meta property="og:type" content={ogType} />
  <meta property="og:url" content={canonicalUrl} />
  <meta property="og:title" content={fullTitle} />
  <meta property="og:description" content={description} />
  <meta property="og:image" content={ogImageUrl} />
  <meta property="og:site_name" content={SITE_NAME} />

  <!-- Twitter -->
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:url" content={canonicalUrl} />
  <meta name="twitter:title" content={fullTitle} />
  <meta name="twitter:description" content={description} />
  <meta name="twitter:image" content={ogImageUrl} />

  <!-- JSON-LD Structured Data -->
  {#if jsonLd}
    {@html `<script type="application/ld+json">${JSON.stringify(jsonLd)}</script>`}
  {/if}
</svelte:head>
```

### Usage Examples

```svelte
<!-- Homepage -->
<SEO
  description="Test your geography knowledge by guessing locations around the world."
  jsonLd={{
    "@context": "https://schema.org",
    "@type": "WebSite",
    "name": "DGuesser",
    "url": "https://dguesser.lol"
  }}
/>

<!-- Map page with dynamic data -->
<SEO
  title={map.name}
  description={map.description || `Play ${map.name} on DGuesser`}
  canonical="/maps/{map.id}"
/>

<!-- Admin page (noindex) -->
<SEO title="Admin" noindex />
```

---

## Task 1.2: Create Server API Helper

**File**: `frontend/src/lib/server/api.ts`

### Purpose

The existing `$lib/api/client.ts` uses `import.meta.env.VITE_API_URL` which only works client-side. For SSR, we need a server-side helper that:

1. Uses environment variables accessible on the server
2. Forwards cookies from the incoming request for authentication
3. Handles errors gracefully for SSR

### Implementation

```typescript
// Server-side API helper for SSR data fetching
import { API_URL } from '$env/static/private';

interface FetchOptions {
  cookies?: string;
}

class ServerApi {
  private baseUrl: string;

  constructor() {
    this.baseUrl = `${API_URL || 'http://localhost:3001'}/api/v1`;
  }

  async get<T>(path: string, options: FetchOptions = {}): Promise<T> {
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };

    if (options.cookies) {
      headers['Cookie'] = options.cookies;
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'GET',
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({
        code: 'UNKNOWN',
        message: 'An unknown error occurred',
      }));
      throw new ServerApiError(response.status, error.code, error.message);
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }
}

export class ServerApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'ServerApiError';
  }
}

export const serverApi = new ServerApi();
```

### Usage in +page.server.ts

```typescript
import { serverApi, ServerApiError } from '$lib/server/api';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ cookies, request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const maps = await serverApi.get<ListMapsResponse>('/maps', {
      cookies: cookieHeader,
    });
    return { maps: maps.maps };
  } catch (e) {
    if (e instanceof ServerApiError && e.status === 404) {
      return { maps: [] };
    }
    throw e;
  }
};
```

---

## Task 1.3: Update app.d.ts

**File**: `frontend/src/app.d.ts`

Add PageData interface for type safety:

```typescript
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    interface PageData {
      // SSR data will be typed per-route
    }
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
```

---

## Task 1.4: Add Environment Variable

**File**: `frontend/.env`

Add:
```bash
# Server-side API URL (same as VITE_API_URL but accessible on server)
API_URL=http://localhost:3001

# Public site URL for canonical links
PUBLIC_SITE_URL=https://dguesser.lol
```

---

## Verification Checklist

- [ ] SEO.svelte renders all meta tags correctly
- [ ] SEO component accepts all documented props
- [ ] Server API helper can fetch data with cookies
- [ ] Server API helper handles errors gracefully
- [ ] Environment variables configured

## Files Changed

| File | Action |
|------|--------|
| `$lib/components/SEO.svelte` | Create |
| `$lib/server/api.ts` | Create |
| `src/app.d.ts` | Modify |
| `.env` | Modify |
