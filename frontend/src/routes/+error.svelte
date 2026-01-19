<script lang="ts">
  import { page } from '$app/stores';
  import { Button } from '$lib/components/ui/button';
  import SEO from '$lib/components/SEO.svelte';

  import MapPinOff from '@lucide/svelte/icons/map-pin-off';
  import Lock from '@lucide/svelte/icons/lock';
  import ServerCrash from '@lucide/svelte/icons/server-crash';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import Home from '@lucide/svelte/icons/home';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';

  const errorConfig = {
    404: {
      title: 'Location Not Found',
      message:
        "We couldn't find the coordinates you're looking for. The page might have moved or the map hasn't been updated.",
      icon: MapPinOff
    },
    403: {
      title: 'Restricted Territory',
      message: "You don't have the necessary clearance to explore this region.",
      icon: Lock
    },
    500: {
      title: 'System Malfunction',
      message:
        "Our navigation systems are experiencing technical difficulties. We're recalibrating—please try again shortly.",
      icon: ServerCrash
    }
  };

  const status = $derived($page.status);
  const errorMessage = $derived($page.error?.message);

  const activeConfig = $derived(
    errorConfig[status as keyof typeof errorConfig] ?? {
      title: 'Navigation Error',
      message: 'An unexpected error occurred while routing your request.',
      icon: TriangleAlert
    }
  );

  const Icon = $derived(activeConfig.icon);

  function goBack() {
    history.back();
  }
</script>

<SEO title="{status} - {activeConfig.title}" description={activeConfig.message} />

<div
  class="flex min-h-[calc(100vh-140px)] flex-col items-center justify-center bg-background px-4 py-16 text-center"
>
  <!-- Status Icon -->
  <div
    class="mb-8 flex size-24 items-center justify-center rounded-3xl bg-muted/40 text-muted-foreground shadow-sm ring-1 ring-inset ring-border/50"
  >
    <Icon class="size-10 opacity-80" />
  </div>

  <!-- Status Code -->
  <span class="mb-3 font-mono text-sm font-semibold tracking-wider text-primary uppercase">
    Error {status}
  </span>

  <!-- Title -->
  <h1 class="mb-4 text-3xl font-bold tracking-tight text-foreground sm:text-4xl">
    {activeConfig.title}
  </h1>

  <!-- Message -->
  <p class="mb-8 max-w-[480px] text-lg text-muted-foreground leading-relaxed">
    {activeConfig.message}
  </p>

  <!-- Technical Details -->
  {#if errorMessage && errorMessage !== activeConfig.message && status !== 404}
    <div
      class="mb-8 max-w-[600px] rounded-md bg-destructive/5 border border-destructive/10 px-4 py-3 font-mono text-xs text-muted-foreground/80 break-all"
    >
      {errorMessage}
    </div>
  {/if}

  <!-- Actions -->
  <div class="flex flex-col gap-3 sm:flex-row">
    <Button href="/" size="lg" class="min-w-[140px] font-medium shadow-sm">
      <Home class="mr-2 size-4" />
      Return Home
    </Button>
    <Button variant="outline" size="lg" onclick={goBack} class="min-w-[140px]">
      <ArrowLeft class="mr-2 size-4" />
      Go Back
    </Button>
  </div>
</div>
