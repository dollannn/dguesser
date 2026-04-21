<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { getStoredAuthRedirect, startGoogleAuth } from '$lib/auth/oauth';
  import SEO from '$lib/components/SEO.svelte';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import AlertTriangleIcon from '@lucide/svelte/icons/triangle-alert';

  const ERROR_COPY: Record<string, { title: string; message: string }> = {
    AUTH_MERGE_BLOCKED: {
      title: 'Finish your current session first',
      message:
        'To safely transfer your guest progress, leave your active party or multiplayer game first, then try signing in again.',
    },
    AUTH_SESSION_ERROR: {
      title: 'Your sign-in session expired',
      message: 'Please start the sign-in process again.',
    },
    AUTH_ERROR: {
      title: 'We could not complete sign-in',
      message: 'Please try again in a moment.',
    },
    OAUTH_ERROR: {
      title: 'Authentication failed',
      message: 'We could not complete sign-in with your provider. Please try again.',
    },
  };

  let returnTo = $derived(getStoredAuthRedirect());
  let errorCode = $derived($page.url.searchParams.get('code') ?? 'OAUTH_ERROR');
  let copy = $derived(ERROR_COPY[errorCode] ?? ERROR_COPY.OAUTH_ERROR);

  function handleTryAgain() {
    startGoogleAuth(returnTo);
  }

  function handleGoBack() {
    goto(returnTo);
  }
</script>

<SEO title="Sign-In Error" noindex />

<main class="min-h-[60vh] flex items-center justify-center px-4 py-12">
  <Card.Root class="w-full max-w-lg">
    <Card.Header class="text-center space-y-3">
      <div class="mx-auto flex size-12 items-center justify-center rounded-full bg-destructive/10 text-destructive">
        <AlertTriangleIcon class="size-6" />
      </div>
      <div>
        <Card.Title>{copy.title}</Card.Title>
        <Card.Description class="mt-2">{copy.message}</Card.Description>
      </div>
    </Card.Header>

    <Card.Content class="flex flex-col gap-3 sm:flex-row sm:justify-center">
      <Button onclick={handleTryAgain}>Try Again</Button>
      <Button variant="outline" onclick={handleGoBack}>Go Back</Button>
    </Card.Content>
  </Card.Root>
</main>
