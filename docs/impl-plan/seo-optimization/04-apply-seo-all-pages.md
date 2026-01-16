# Phase 4: Apply SEO to All Pages

> Add SEO component with appropriate metadata to every page

## Objectives

Replace existing `<svelte:head>` blocks with the SEO component, ensuring:
- Unique, descriptive titles
- Relevant meta descriptions
- Proper canonical URLs
- Appropriate noindex flags

---

## Page SEO Specifications

### Public Pages (Index)

| Route | Title | Description | Notes |
|-------|-------|-------------|-------|
| `/` | (default) | Test your geography knowledge... | Homepage, WebSite JSON-LD |
| `/play` | Play | Start a new game and test your geography skills | High-value action page |
| `/maps` | Maps | Browse and play custom geography maps... | Discovery page |
| `/maps/[id]` | {map.name} | {map.description} or default | Dynamic from SSR data |
| `/leaderboard` | Leaderboard | See how you rank against other players | Competition page |
| `/u/[username]` | {display_name} (@{username}) | {name}'s DGuesser profile... | Dynamic, Profile JSON-LD |
| `/privacy` | Privacy Policy | How DGuesser handles your data | Legal page |
| `/terms` | Terms of Service | Terms and conditions for using DGuesser | Legal page |

### Private Pages (noindex)

| Route | Title | Notes |
|-------|-------|-------|
| `/game/[id]` | Game | Active games - dynamic, no SEO value |
| `/maps/[id]/edit` | Edit {map.name} | Authenticated only |
| `/maps/new` | Create Map | Authenticated only |
| `/history` | Game History | Personal data |
| `/account` | Account Settings | Personal data |
| `/auth` | Sign In | Auth flow |
| `/auth/success` | - | Auth callback |
| `/admin/*` | Admin... | Admin pages |

---

## Task 4.1: Homepage

**File**: `frontend/src/routes/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  description="Test your geography knowledge by guessing locations around the world. Play solo or compete with friends in this free geography guessing game."
  jsonLd={{
    "@context": "https://schema.org",
    "@type": "WebSite",
    "name": "DGuesser",
    "url": "https://dguesser.lol",
    "description": "A free geography guessing game",
    "potentialAction": {
      "@type": "PlayAction",
      "target": "https://dguesser.lol/play"
    }
  }}
/>
```

---

## Task 4.2: Play Page

**File**: `frontend/src/routes/play/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  title="Play"
  description="Start a new geography guessing game. Choose your map, difficulty, and challenge yourself to identify locations from around the world."
/>
```

---

## Task 4.3: Maps List Page

**File**: `frontend/src/routes/maps/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  title="Maps"
  description="Browse custom geography maps created by the community. Play maps focused on specific countries, cities, landmarks, or themes."
/>
```

---

## Task 4.4: Map Details Page

**File**: `frontend/src/routes/maps/[id]/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
  let map = $state(data.map);

  // Generate description from map data
  let seoDescription = $derived(
    map.description ||
    `Play ${map.name} on DGuesser. ${map.location_count} locations to explore.`
  );
</script>

<SEO
  title={map.name}
  description={seoDescription}
  canonical="/maps/{map.id}"
/>
```

---

## Task 4.5: Leaderboard Page

**File**: `frontend/src/routes/leaderboard/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  title="Leaderboard"
  description="See how you rank against other DGuesser players. View top scores by total points, best game, games played, and average score."
/>
```

---

## Task 4.6: User Profile Page

**File**: `frontend/src/routes/u/[username]/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();
  let profile = $state(data.profile);

  let seoDescription = $derived(
    `${profile.display_name}'s DGuesser profile. ${profile.games_played} games played with a total score of ${profile.total_score.toLocaleString()}.`
  );

  let jsonLd = $derived({
    "@context": "https://schema.org",
    "@type": "ProfilePage",
    "mainEntity": {
      "@type": "Person",
      "name": profile.display_name,
      "identifier": profile.username,
      ...(profile.avatar_url && { "image": profile.avatar_url })
    }
  });
</script>

<SEO
  title="{profile.display_name} (@{profile.username})"
  description={seoDescription}
  canonical="/u/{profile.username}"
  ogType="profile"
  jsonLd={jsonLd}
/>
```

---

## Task 4.7: Privacy & Terms Pages

**File**: `frontend/src/routes/privacy/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  title="Privacy Policy"
  description="Learn how DGuesser collects, uses, and protects your personal information."
/>
```

**File**: `frontend/src/routes/terms/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<SEO
  title="Terms of Service"
  description="Read the terms and conditions for using DGuesser, including user responsibilities and service limitations."
/>
```

---

## Task 4.8: noindex Pages

### Game Page
**File**: `frontend/src/routes/game/[id]/+page.svelte`

```svelte
<script lang="ts">
  import SEO from '$lib/components/SEO.svelte';
</script>

<!-- Fix: "dguesser" -> "DGuesser" in title -->
<SEO title="Game" noindex />
```

### Map Editor
**File**: `frontend/src/routes/maps/[id]/edit/+page.svelte`

```svelte
<SEO title="Edit {map.name}" noindex />
```

### Create Map
**File**: `frontend/src/routes/maps/new/+page.svelte`

```svelte
<SEO title="Create Map" noindex />
```

### Game History
**File**: `frontend/src/routes/history/+page.svelte`

```svelte
<SEO title="Game History" noindex />
```

### Account Settings
**File**: `frontend/src/routes/account/+page.svelte`

```svelte
<SEO title="Account Settings" noindex />
```

### Auth Pages
**File**: `frontend/src/routes/auth/+page.svelte`

```svelte
<SEO title="Sign In" noindex />
```

### Admin Pages
**Files**: All `/admin/*` pages

```svelte
<SEO title="Admin" noindex />
```

---

## Task 4.9: Update Root Layout

**File**: `frontend/src/routes/+layout.svelte`

Remove the existing `<svelte:head>` block since the SEO component handles defaults:

```diff
- <svelte:head>
-     <title>DGuesser - Geography Guessing Game</title>
-     <meta
-         name="description"
-         content="Test your geography knowledge by guessing locations around the world. Play solo or compete with friends!"
-     />
- </svelte:head>
```

The SEO component will provide default title/description when not overridden by individual pages.

---

## Description Guidelines

### Length
- **Optimal**: 150-160 characters
- **Minimum**: 50 characters
- **Maximum**: 160 characters (truncated in search results)

### Content
- Include primary keyword naturally
- Describe page content accurately
- Include call-to-action when appropriate
- Make each description unique

### Examples

**Good**:
> "Test your geography knowledge by guessing locations around the world. Play solo or compete with friends in this free geography guessing game."

**Bad**:
> "DGuesser is a game where you guess things on maps and stuff."

---

## Verification Checklist

- [ ] All public pages have SEO component
- [ ] All private pages have `noindex`
- [ ] Descriptions are unique and within length limits
- [ ] Dynamic pages use SSR data for SEO
- [ ] Title casing is consistent ("DGuesser" not "dguesser")
- [ ] Root layout doesn't duplicate meta tags

## Testing

Use browser DevTools to inspect `<head>`:
```javascript
document.querySelectorAll('meta').forEach(m => console.log(m.outerHTML));
```

Or view page source (Ctrl+U) to see SSR output.

## Files Changed

| File | Action |
|------|--------|
| `routes/+page.svelte` | Modify |
| `routes/+layout.svelte` | Modify |
| `routes/play/+page.svelte` | Modify |
| `routes/maps/+page.svelte` | Modify |
| `routes/maps/[id]/+page.svelte` | Modify |
| `routes/maps/[id]/edit/+page.svelte` | Modify |
| `routes/maps/new/+page.svelte` | Modify |
| `routes/leaderboard/+page.svelte` | Modify |
| `routes/u/[username]/+page.svelte` | Modify |
| `routes/game/[id]/+page.svelte` | Modify |
| `routes/history/+page.svelte` | Modify |
| `routes/account/+page.svelte` | Modify |
| `routes/auth/+page.svelte` | Modify |
| `routes/privacy/+page.svelte` | Modify |
| `routes/terms/+page.svelte` | Modify |
| `routes/admin/+page.svelte` | Modify |
| `routes/admin/*/+page.svelte` | Modify |
