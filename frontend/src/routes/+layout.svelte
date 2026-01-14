<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { authStore, isLoading } from '$lib/stores/auth';
  import { socketClient } from '$lib/socket/client';
  import Header from '$lib/components/Header.svelte';

  let { children } = $props();

  onMount(() => {
    // Initialize auth on mount
    authStore.initialize();
    
    // Connect to realtime if authenticated
    const unsubscribe = authStore.subscribe(($auth) => {
      if ($auth.user && !socketClient.connected) {
        socketClient.connect();
      }
    });

    return () => {
      unsubscribe();
      socketClient.disconnect();
    };
  });
</script>

<svelte:head>
  <title>DGuesser</title>
</svelte:head>

<div class="min-h-screen flex flex-col">
  <Header />
  
  <main class="flex-1">
    {#if $isLoading}
      <div class="flex items-center justify-center h-64">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    {:else}
      {@render children()}
    {/if}
  </main>

  <footer class="bg-gray-100 py-4 text-center text-sm text-gray-600">
    <p>dguesser - A geography guessing game</p>
  </footer>
</div>
