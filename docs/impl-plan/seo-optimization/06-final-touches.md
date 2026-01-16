# Phase 6: Final Touches

> Complete the SEO implementation with robots.txt, prerendering, and cleanup

## Objectives

1. Update robots.txt with sitemap and exclusions
2. Prerender static pages for performance
3. Ensure all noindex pages are properly configured
4. Final verification and testing

---

## Task 6.1: Update robots.txt

**File**: `frontend/static/robots.txt`

### Current State
```
User-agent: *
Disallow:
```

### Updated Version
```
# DGuesser robots.txt
# https://dguesser.lol

User-agent: *

# Block authentication flows
Disallow: /auth/
Disallow: /auth

# Block active games (dynamic, no SEO value)
Disallow: /game/

# Block admin section
Disallow: /admin/
Disallow: /admin

# Block user-specific pages
Disallow: /account
Disallow: /history

# Block map editing (authenticated)
Disallow: /maps/*/edit

# Block map creation (authenticated)
Disallow: /maps/new

# Sitemap location
Sitemap: https://dguesser.lol/sitemap.xml
```

### Explanation

| Path | Reason |
|------|--------|
| `/auth/` | Authentication flow, no content |
| `/game/` | Active games are ephemeral |
| `/admin/` | Admin-only content |
| `/account` | Personal user settings |
| `/history` | Personal game history |
| `/maps/*/edit` | Authenticated editing |
| `/maps/new` | Authenticated creation |

---

## Task 6.2: Prerender Static Pages

Static pages that never change can be prerendered at build time.

### Privacy Page

**File**: `frontend/src/routes/privacy/+page.ts`

```typescript
export const prerender = true;
```

### Terms Page

**File**: `frontend/src/routes/terms/+page.ts`

```typescript
export const prerender = true;
```

### Benefits
- Faster load times (no server rendering)
- Can be served from CDN edge
- Reduces server load

---

## Task 6.3: Verify noindex Implementation

Ensure all private pages have the noindex meta tag:

### Checklist

| Page | Has noindex | Verified |
|------|-------------|----------|
| `/game/[id]` | Yes | [ ] |
| `/maps/[id]/edit` | Yes | [ ] |
| `/maps/new` | Yes | [ ] |
| `/history` | Yes | [ ] |
| `/account` | Yes | [ ] |
| `/auth` | Yes | [ ] |
| `/auth/success` | Yes | [ ] |
| `/admin` | Yes | [ ] |
| `/admin/locations` | Yes | [ ] |
| `/admin/locations/[id]` | Yes | [ ] |
| `/admin/reports` | Yes | [ ] |

### Verification Command
```bash
# Check for noindex on a page
curl -s https://dguesser.lol/admin | grep -i 'noindex'
```

---

## Task 6.4: Fix Title Inconsistency

**File**: `frontend/src/routes/game/[id]/+page.svelte`

Change:
```svelte
<title>Game - dguesser</title>
```

To (using SEO component):
```svelte
<SEO title="Game" noindex />
```

This ensures consistent "DGuesser" branding.

---

## Task 6.5: Add theme-color Meta Tag

**File**: `frontend/src/app.html`

Add to `<head>`:
```html
<meta name="theme-color" content="#1f2937" />
```

This sets the browser chrome color on mobile devices.

---

## Task 6.6: Final Testing Checklist

### Technical SEO
- [ ] All pages have unique titles
- [ ] All pages have meta descriptions
- [ ] All pages have canonical URLs
- [ ] robots.txt is accessible at /robots.txt
- [ ] sitemap.xml is accessible at /sitemap.xml
- [ ] sitemap.xml is valid XML
- [ ] No broken internal links

### Social Sharing
- [ ] Homepage shares correctly on Twitter
- [ ] Homepage shares correctly on Facebook
- [ ] Map pages show correct preview
- [ ] User profiles show correct preview

### Search Console
- [ ] Submit sitemap to Google Search Console
- [ ] Submit sitemap to Bing Webmaster Tools
- [ ] Check for crawl errors
- [ ] Request indexing for key pages

### Performance
- [ ] Lighthouse SEO score > 90
- [ ] Core Web Vitals pass
- [ ] Mobile-friendly test passes

---

## Task 6.7: Google Search Console Setup

1. Go to https://search.google.com/search-console
2. Add property: `https://dguesser.lol`
3. Verify ownership (DNS, HTML file, or meta tag)
4. Submit sitemap: `https://dguesser.lol/sitemap.xml`
5. Monitor coverage and performance

---

## Task 6.8: Social Sharing Preview Testing

### Facebook Debugger
https://developers.facebook.com/tools/debug/

### Twitter Card Validator
https://cards-dev.twitter.com/validator

### LinkedIn Post Inspector
https://www.linkedin.com/post-inspector/

### Open Graph Preview
https://www.opengraph.xyz/

---

## Lighthouse SEO Audit

Run Lighthouse in Chrome DevTools or via CLI:

```bash
npx lighthouse https://dguesser.lol --only-categories=seo --output=html --output-path=./seo-report.html
```

### Common Issues to Fix
- Missing meta description (should be fixed)
- Links not crawlable (ensure `<a href>` not JS navigation)
- Images without alt text
- Document doesn't have a `<title>` element

---

## Monitoring

### Weekly Checks
- Google Search Console coverage report
- New crawl errors
- Index status

### Monthly Checks
- Lighthouse SEO score
- Search performance metrics
- Click-through rates

---

## Files Changed

| File | Action |
|------|--------|
| `static/robots.txt` | Modify |
| `routes/privacy/+page.ts` | Create |
| `routes/terms/+page.ts` | Create |
| `routes/game/[id]/+page.svelte` | Modify (title fix) |
| `src/app.html` | Modify (theme-color) |

---

## Post-Implementation

After all phases are complete:

1. **Deploy to production**
2. **Submit sitemap to search engines**
3. **Test social sharing on all platforms**
4. **Monitor Search Console for issues**
5. **Track organic traffic changes**

### Expected Timeline for Results
- Indexing: 1-4 weeks
- Ranking improvements: 1-3 months
- Full SEO impact: 3-6 months
