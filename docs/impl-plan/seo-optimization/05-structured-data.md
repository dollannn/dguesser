# Phase 5: Structured Data (JSON-LD)

> Add schema.org structured data for rich search results

## Objectives

Implement JSON-LD structured data to:
- Enable rich snippets in search results
- Provide context about the site and content to search engines
- Potentially enable special search features (sitelinks, knowledge panel)

---

## Schema Types to Implement

| Schema | Page | Purpose |
|--------|------|---------|
| WebSite | Homepage | Site search box, brand identity |
| Organization | Layout (all pages) | Brand information |
| ProfilePage | User profiles | Person information |
| WebPage | Default for all pages | Basic page info |

---

## Task 5.1: Organization Schema (Global)

**File**: `frontend/src/routes/+layout.svelte`

Add Organization schema that appears on all pages:

```svelte
<script lang="ts">
  // ... existing imports

  const organizationSchema = {
    "@context": "https://schema.org",
    "@type": "Organization",
    "name": "DGuesser",
    "url": "https://dguesser.lol",
    "logo": "https://dguesser.lol/favicon.svg",
    "sameAs": [
      "https://github.com/dollannn/dguesser"
    ],
    "description": "A free geography guessing game where players identify locations from around the world."
  };
</script>

<svelte:head>
  {@html `<script type="application/ld+json">${JSON.stringify(organizationSchema)}</script>`}
</svelte:head>
```

---

## Task 5.2: WebSite Schema (Homepage)

**File**: `frontend/src/routes/+page.svelte`

Already included in Phase 4, but here's the full schema:

```typescript
const websiteSchema = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  "name": "DGuesser",
  "alternateName": "D Guesser",
  "url": "https://dguesser.lol",
  "description": "A free geography guessing game where players identify locations from around the world",
  "potentialAction": {
    "@type": "SearchAction",
    "target": {
      "@type": "EntryPoint",
      "urlTemplate": "https://dguesser.lol/maps?q={search_term_string}"
    },
    "query-input": "required name=search_term_string"
  }
};
```

**Note**: The SearchAction enables Google's site search box if your site qualifies. Requires a working search feature at the target URL.

---

## Task 5.3: ProfilePage Schema (User Profiles)

**File**: `frontend/src/routes/u/[username]/+page.svelte`

Already included in Phase 4:

```typescript
const profileSchema = {
  "@context": "https://schema.org",
  "@type": "ProfilePage",
  "mainEntity": {
    "@type": "Person",
    "name": profile.display_name,
    "identifier": profile.username,
    "image": profile.avatar_url || undefined,
    "interactionStatistic": [
      {
        "@type": "InteractionCounter",
        "interactionType": "https://schema.org/PlayAction",
        "userInteractionCount": profile.games_played
      }
    ]
  }
};
```

---

## Task 5.4: Game/VideoGame Schema (Optional)

For individual maps, you could add VideoGame schema:

**File**: `frontend/src/routes/maps/[id]/+page.svelte`

```typescript
const gameSchema = {
  "@context": "https://schema.org",
  "@type": "VideoGame",
  "name": map.name,
  "description": map.description,
  "url": `https://dguesser.lol/maps/${map.id}`,
  "genre": "Geography",
  "numberOfPlayers": {
    "@type": "QuantitativeValue",
    "minValue": 1,
    "maxValue": 1
  },
  "gamePlatform": "Web Browser",
  "applicationCategory": "Game"
};
```

**Note**: This is optional and may not provide significant SEO benefit for a small site.

---

## Task 5.5: BreadcrumbList Schema (Optional)

For navigation context:

```typescript
// On /maps/[id] page
const breadcrumbSchema = {
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  "itemListElement": [
    {
      "@type": "ListItem",
      "position": 1,
      "name": "Home",
      "item": "https://dguesser.lol"
    },
    {
      "@type": "ListItem",
      "position": 2,
      "name": "Maps",
      "item": "https://dguesser.lol/maps"
    },
    {
      "@type": "ListItem",
      "position": 3,
      "name": map.name,
      "item": `https://dguesser.lol/maps/${map.id}`
    }
  ]
};
```

---

## Schema Validation

### Google Rich Results Test
https://search.google.com/test/rich-results

### Schema.org Validator
https://validator.schema.org/

### Test Your Markup
```bash
# Fetch page and extract JSON-LD
curl -s https://dguesser.lol | grep -o '<script type="application/ld+json">.*</script>'
```

---

## Important Notes

### Multiple Schemas
You can have multiple JSON-LD blocks on a page. Each should be a complete, valid schema.

### Nesting
Keep schemas simple. Deep nesting can cause validation issues.

### Required vs Recommended Properties

**Organization** (Required):
- name
- url

**Organization** (Recommended):
- logo
- description
- sameAs

**WebSite** (Required):
- name
- url

**ProfilePage** (Required):
- mainEntity

---

## Verification Checklist

- [ ] Organization schema on all pages
- [ ] WebSite schema on homepage
- [ ] ProfilePage schema on user profiles
- [ ] All schemas validate at schema.org
- [ ] Google Rich Results Test passes
- [ ] No duplicate schemas

## Testing Commands

```bash
# Check for JSON-LD presence
curl -s https://dguesser.lol | grep -c 'application/ld+json'

# Extract and pretty-print JSON-LD
curl -s https://dguesser.lol | \
  grep -o '<script type="application/ld+json">[^<]*</script>' | \
  sed 's/<[^>]*>//g' | \
  jq .
```

## Files Changed

| File | Action |
|------|--------|
| `routes/+layout.svelte` | Modify (Organization) |
| `routes/+page.svelte` | Already done (WebSite) |
| `routes/u/[username]/+page.svelte` | Already done (ProfilePage) |
| `routes/maps/[id]/+page.svelte` | Optional (VideoGame) |
