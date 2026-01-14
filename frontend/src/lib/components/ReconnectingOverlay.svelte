<script lang="ts">
  import { socketClient } from '$lib/socket/client';

  const state = socketClient.state;

  function handleCancel() {
    socketClient.disconnect();
  }
</script>

{#if $state.status === 'reconnecting'}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="reconnecting-title"
  >
    <div class="bg-white rounded-xl shadow-2xl p-8 max-w-sm mx-4 text-center">
      <!-- Spinner -->
      <div class="mb-6">
        <svg
          class="animate-spin h-12 w-12 text-primary-600 mx-auto"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            class="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            stroke-width="4"
          ></circle>
          <path
            class="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          ></path>
        </svg>
      </div>

      <h2 id="reconnecting-title" class="text-xl font-semibold text-gray-900 mb-2">
        Reconnecting...
      </h2>

      <p class="text-gray-600 mb-4">
        Attempting to restore your connection
      </p>

      <div class="text-sm text-gray-500 mb-6">
        Attempt {$state.reconnectAttempt} of {$state.maxReconnectAttempts}
      </div>

      <!-- Progress bar -->
      <div class="w-full bg-gray-200 rounded-full h-1.5 mb-6">
        <div
          class="bg-primary-600 h-1.5 rounded-full transition-all duration-300"
          style="width: {($state.reconnectAttempt / $state.maxReconnectAttempts) * 100}%"
        ></div>
      </div>

      <button
        type="button"
        class="btn-secondary text-sm"
        onclick={handleCancel}
      >
        Cancel
      </button>
    </div>
  </div>
{/if}
