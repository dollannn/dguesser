# Phase 2: SSR Data Loading

> Add server-side data loading for search engine visibility

## Objectives

Enable search engines to see actual content on data-driven pages by loading data server-side.

**Why this matters**: Currently, pages like `/maps` and `/leaderboard` load data in `$effect()` hooks. Search engine crawlers often don't execute JavaScript, so they see empty content. Server-side loading ensures crawlers see the full page.

---

## Task 2.1: Maps List Page

**File**: `frontend/src/routes/maps/+page.server.ts`

### Implementation

```typescript
import { serverApi, ServerApiError } from '$lib/server/api';
import type { PageServerLoad } from './$types';

interface MapSummary {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: 'private' | 'unlisted' | 'public';
  is_system_map: boolean;
  is_owned: boolean;
  location_count: number;
  created_at: string;
}

interface ListMapsResponse {
  maps: MapSummary[];
}

export const load: PageServerLoad = async ({ request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const response = await serverApi.get<ListMapsResponse>('/maps', {
      cookies: cookieHeader,
    });
    return {
      maps: response.maps,
      error: null,
    };
  } catch (e) {
    console.error('Failed to load maps:', e);
    return {
      maps: [],
      error: e instanceof ServerApiError ? e.message : 'Failed to load maps',
    };
  }
};
```

### Update +page.svelte

Modify the page to use server data as initial state:

```svelte
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Initialize from server data
  let maps = $state(data.maps);
  let loading = $state(false);  // No longer loading initially
  let error = $state(data.error || '');

  // ... rest of component logic unchanged
  // Remove the $effect() that calls loadMaps() on mount
  // Keep loadMaps() for filter changes if needed
</script>
```

---

## Task 2.2: Map Details Page

**File**: `frontend/src/routes/maps/[id]/+page.server.ts`

### Implementation

```typescript
import { serverApi, ServerApiError } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

interface MapDetails {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: 'private' | 'unlisted' | 'public';
  is_system_map: boolean;
  is_owned: boolean;
  is_default: boolean;
  location_count: number;
  created_at: string;
  updated_at: string;
}

export const load: PageServerLoad = async ({ params, request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const map = await serverApi.get<MapDetails>(`/maps/${params.id}`, {
      cookies: cookieHeader,
    });
    return { map };
  } catch (e) {
    if (e instanceof ServerApiError) {
      if (e.status === 404) {
        throw error(404, 'Map not found');
      }
      if (e.status === 403) {
        throw error(403, 'You do not have access to this map');
      }
    }
    throw error(500, 'Failed to load map');
  }
};
```

### Update +page.svelte

```svelte
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Map is now available immediately from SSR
  let map = $state(data.map);
  let loading = $state(false);

  // Remove $effect() that loaded map on mount
</script>
```

---

## Task 2.3: Leaderboard Page

**File**: `frontend/src/routes/leaderboard/+page.server.ts`

### Implementation

```typescript
import { serverApi } from '$lib/server/api';
import type { PageServerLoad } from './$types';

interface LeaderboardEntry {
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  rank: number;
  score: number;
  games_played: number;
  is_current_user: boolean;
}

interface LeaderboardResponse {
  entries: LeaderboardEntry[];
  total_players: number;
  current_user_rank: number | null;
  current_user_score: number | null;
}

export const load: PageServerLoad = async ({ request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const response = await serverApi.get<LeaderboardResponse>(
      '/leaderboard?type=total_score&period=all_time&limit=50',
      { cookies: cookieHeader }
    );
    return {
      initialEntries: response.entries,
      totalPlayers: response.total_players,
      currentUserRank: response.current_user_rank,
      currentUserScore: response.current_user_score,
    };
  } catch (e) {
    console.error('Failed to load leaderboard:', e);
    return {
      initialEntries: [],
      totalPlayers: 0,
      currentUserRank: null,
      currentUserScore: null,
    };
  }
};
```

### Update +page.svelte

```svelte
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Initialize from server data
  let entries = $state(data.initialEntries);
  let totalPlayers = $state(data.totalPlayers);
  let currentUserRank = $state(data.currentUserRank);
  let currentUserScore = $state(data.currentUserScore);

  // Loading is false initially since we have SSR data
  let loading = $state(false);
  let hasLoadedOnce = $state(true);  // Already loaded via SSR

  // Filter changes still trigger client-side fetches
  // ... keep existing filter logic
</script>
```

---

## Task 2.4: User Profile Page

**File**: `frontend/src/routes/u/[username]/+page.server.ts`

### Implementation

```typescript
import { serverApi, ServerApiError } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

interface UserProfile {
  id: string;
  username: string;
  display_name: string;
  avatar_url: string | null;
  is_guest: boolean;
  games_played: number;
  total_score: number;
  best_score: number;
}

export const load: PageServerLoad = async ({ params, request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const profile = await serverApi.get<UserProfile>(
      `/users/username/${params.username}`,
      { cookies: cookieHeader }
    );
    return { profile };
  } catch (e) {
    if (e instanceof ServerApiError && e.status === 404) {
      throw error(404, 'User not found');
    }
    throw error(500, 'Failed to load profile');
  }
};
```

### Update +page.svelte

```svelte
<script lang="ts">
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Profile available immediately from SSR
  let profile = $state(data.profile);
  let loading = $state(false);
  let error = $state('');

  // Remove $effect() that loaded profile on mount
</script>
```

---

## Task 2.5: Type Definitions

Create shared types file if not exists:

**File**: `frontend/src/lib/types/api.ts`

```typescript
// Re-export types for use in both client and server
export type { MapSummary, MapDetails, ListMapsResponse } from '$lib/api/maps';
export type { UserProfile } from '$lib/api/users';
export type { LeaderboardEntry, LeaderboardResponse } from '$lib/api/leaderboard';
```

---

## Important Considerations

### Hydration
- Server data becomes initial state
- Client-side interactions (filtering, pagination) still work via client API
- State updates seamlessly after hydration

### Error Handling
- Server errors result in error pages (404, 500)
- Graceful fallbacks for non-critical errors

### Performance
- Initial page load is slower (server fetches data)
- But users see content immediately (no loading spinner)
- Crawlers see full content

### Authentication
- Cookies are forwarded to API
- Authenticated users see their data
- Anonymous users see public content

---

## Verification Checklist

- [ ] `/maps` shows maps without JavaScript
- [ ] `/maps/[id]` shows map details without JavaScript
- [ ] `/leaderboard` shows entries without JavaScript
- [ ] `/u/[username]` shows profile without JavaScript
- [ ] Error pages work correctly (404, 403)
- [ ] Client-side filtering still works after hydration
- [ ] Authenticated content loads correctly

## Testing SSR

```bash
# Disable JavaScript in browser DevTools
# Or use curl to see what crawlers see:
curl -s https://dguesser.lol/maps | grep -o '<h1>.*</h1>'
curl -s https://dguesser.lol/leaderboard | grep -o '<title>.*</title>'
```

## Files Changed

| File | Action |
|------|--------|
| `routes/maps/+page.server.ts` | Create |
| `routes/maps/[id]/+page.server.ts` | Create |
| `routes/leaderboard/+page.server.ts` | Create |
| `routes/u/[username]/+page.server.ts` | Create |
| `routes/maps/+page.svelte` | Modify |
| `routes/maps/[id]/+page.svelte` | Modify |
| `routes/leaderboard/+page.svelte` | Modify |
| `routes/u/[username]/+page.svelte` | Modify |
