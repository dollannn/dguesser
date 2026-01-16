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
  const DEFAULT_DESCRIPTION =
    'Test your geography knowledge by guessing locations around the world. Play solo or compete with friends!';
  const DEFAULT_OG_IMAGE = '/favicon.svg';

  let {
    title,
    description = DEFAULT_DESCRIPTION,
    canonical,
    ogImage = DEFAULT_OG_IMAGE,
    ogType = 'website',
    noindex = false,
    jsonLd
  }: Props = $props();

  // Compute full title
  let fullTitle = $derived(
    title ? `${title} - ${SITE_NAME}` : `${SITE_NAME} - Geography Guessing Game`
  );

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
