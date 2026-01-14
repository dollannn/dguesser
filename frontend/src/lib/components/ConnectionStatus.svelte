<script lang="ts">
  import { socketClient, type ConnectionStatus } from '$lib/socket/client';

  const state = socketClient.state;

  function getStatusColor(status: ConnectionStatus): string {
    switch (status) {
      case 'authenticated':
      case 'connected':
        return 'bg-green-500';
      case 'reconnecting':
      case 'connecting':
        return 'bg-yellow-500';
      case 'disconnected':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  }

  function getStatusText(status: ConnectionStatus, attempt: number, max: number): string {
    switch (status) {
      case 'authenticated':
        return 'Connected';
      case 'connected':
        return 'Authenticating...';
      case 'reconnecting':
        return `Reconnecting (${attempt}/${max})`;
      case 'connecting':
        return 'Connecting...';
      case 'disconnected':
        return 'Disconnected';
      default:
        return 'Unknown';
    }
  }

  function handleClick() {
    if ($state.status === 'disconnected') {
      socketClient.reconnect();
    }
  }
</script>

<button
  type="button"
  class="flex items-center gap-2 px-3 py-1.5 rounded-full text-sm font-medium transition-all
    {$state.status === 'disconnected' ? 'cursor-pointer hover:bg-gray-100' : 'cursor-default'}
    {$state.status === 'reconnecting' ? 'animate-pulse' : ''}"
  onclick={handleClick}
  disabled={$state.status !== 'disconnected'}
>
  <span
    class="w-2.5 h-2.5 rounded-full {getStatusColor($state.status)}
      {$state.status === 'reconnecting' || $state.status === 'connecting' ? 'animate-pulse' : ''}"
  ></span>
  <span class="text-gray-700">
    {getStatusText($state.status, $state.reconnectAttempt, $state.maxReconnectAttempts)}
  </span>
  {#if $state.status === 'disconnected' && !$state.error}
    <span class="text-xs text-gray-500">(click to reconnect)</span>
  {/if}
</button>

{#if $state.error}
  <span class="text-xs text-red-600 ml-2">{$state.error}</span>
{/if}
