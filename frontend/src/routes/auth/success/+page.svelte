<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { authStore } from '$lib/stores/auth';

  onMount(() => {
    // Refresh user data after OAuth callback
    authStore.initialize().then(() => {
      // Redirect to home or stored redirect
      const redirectTo = sessionStorage.getItem('auth_redirect') || '/';
      sessionStorage.removeItem('auth_redirect');
      goto(redirectTo);
    });
  });
</script>

<div class="flex items-center justify-center min-h-[60vh]">
  <div class="text-center">
    <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto mb-4"></div>
    <p class="text-gray-600">Completing sign in...</p>
  </div>
</div>
