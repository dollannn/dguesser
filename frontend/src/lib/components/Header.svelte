<script lang="ts">
  import { user, isGuest, authStore } from '$lib/stores/auth';
  import { authApi } from '$lib/api/auth';

  async function handleLogout() {
    await authStore.logout();
  }
</script>

<header class="bg-white shadow-sm">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="flex justify-between items-center h-16">
      <a href="/" class="text-2xl font-bold text-primary-600">
        dguesser
      </a>

      <nav class="flex items-center gap-4">
        {#if $user}
          <a href="/play" class="text-gray-600 hover:text-gray-900">
            Play
          </a>
          <a href="/history" class="text-gray-600 hover:text-gray-900">
            History
          </a>
          
          <div class="flex items-center gap-3 ml-4">
            {#if $user.avatar_url}
              <img 
                src={$user.avatar_url} 
                alt={$user.display_name}
                class="w-8 h-8 rounded-full"
              />
            {:else}
              <div class="w-8 h-8 rounded-full bg-primary-100 flex items-center justify-center">
                <span class="text-primary-600 font-medium">
                  {$user.display_name.charAt(0).toUpperCase()}
                </span>
              </div>
            {/if}
            
            <span class="text-sm font-medium text-gray-700">
              {$user.display_name}
              {#if $isGuest}
                <span class="text-xs text-gray-500">(Guest)</span>
              {/if}
            </span>

            {#if $isGuest}
              <a 
                href={authApi.getGoogleAuthUrl()}
                class="btn-primary text-sm"
              >
                Sign In
              </a>
            {:else}
              <button 
                onclick={handleLogout}
                class="btn-secondary text-sm"
              >
                Logout
              </button>
            {/if}
          </div>
        {:else}
          <a 
            href={authApi.getGoogleAuthUrl()}
            class="btn-primary"
          >
            Sign In with Google
          </a>
        {/if}
      </nav>
    </div>
  </div>
</header>
