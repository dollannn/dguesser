<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { authStore, isLoading } from '$lib/stores/auth';
  import { socketClient } from '$lib/socket/client';
  import { initGameSocketListeners } from '$lib/socket/game';
  import Header from '$lib/components/Header.svelte';
  import AuthModal from '$lib/components/AuthModal.svelte';
  import ReconnectingOverlay from '$lib/components/ReconnectingOverlay.svelte';
  import { Toaster } from '$lib/components/ui/sonner';
  import { Separator } from '$lib/components/ui/separator';
  import { Spinner } from '$lib/components/ui/spinner';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import GithubIcon from '@lucide/svelte/icons/github';
  import HeartIcon from '@lucide/svelte/icons/heart';

  let { children } = $props();

  onMount(() => {
    authStore.initialize();

    const cleanupListeners = initGameSocketListeners();
    
    const unsubscribe = authStore.subscribe(($auth) => {
      if ($auth.user && !socketClient.connected) {
        socketClient.connect();
      }
    });

    return () => {
      cleanupListeners();
      unsubscribe();
      socketClient.disconnect();
    };
  });
</script>

<svelte:head>
  <title>DGuesser - Geography Guessing Game</title>
  <meta name="description" content="Test your geography knowledge by guessing locations around the world. Play solo or compete with friends!" />
</svelte:head>

<Tooltip.Provider>
<div class="min-h-screen flex flex-col">
  <Header />
  
  <main class="flex-1">
    {#if $isLoading}
      <div class="flex items-center justify-center h-64">
        <Spinner class="size-10 text-primary" />
      </div>
    {:else}
      {@render children()}
    {/if}
  </main>

  <footer class="border-t bg-muted/30">
    <div class="container mx-auto px-4 py-8">
      <div class="flex flex-col md:flex-row items-center justify-between gap-6">
        <div class="flex items-center gap-2 text-muted-foreground">
          <GlobeIcon class="size-5" />
          <span class="font-semibold">DGuesser</span>
          <span class="text-sm">- A geography guessing game</span>
        </div>
        
        <nav class="flex items-center gap-6 text-sm text-muted-foreground">
          <a href="/terms" class="hover:text-foreground transition-colors">
            Terms
          </a>
          <a href="/privacy" class="hover:text-foreground transition-colors">
            Privacy
          </a>
          <Separator orientation="vertical" class="h-4" />
          <a 
            href="https://github.com" 
            target="_blank" 
            rel="noopener noreferrer"
            class="hover:text-foreground transition-colors flex items-center gap-1"
          >
            <GithubIcon class="size-4" />
            GitHub
          </a>
        </nav>
      </div>
      
      <Separator class="my-6" />
      
      <div class="flex flex-col md:flex-row items-center justify-between gap-4 text-sm text-muted-foreground">
        <p>&copy; {new Date().getFullYear()} DGuesser. All rights reserved.</p>
        <p class="flex items-center gap-1">
          Made with <HeartIcon class="size-4 text-red-500 fill-red-500" /> for geography enthusiasts
        </p>
      </div>
    </div>
  </footer>
</div>

<!-- Global overlays and notifications -->
<AuthModal />
<ReconnectingOverlay />
<Toaster richColors closeButton />
</Tooltip.Provider>
