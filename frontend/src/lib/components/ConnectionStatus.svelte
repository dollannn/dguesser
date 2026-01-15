<script lang="ts">
  import { socketClient, type ConnectionStatus } from '$lib/socket/client';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import WifiIcon from '@lucide/svelte/icons/wifi';
  import WifiOffIcon from '@lucide/svelte/icons/wifi-off';
  import LoaderIcon from '@lucide/svelte/icons/loader';

  interface Props {
    minimal?: boolean;
  }

  let { minimal = false }: Props = $props();

  const state = socketClient.state;

  function getStatusInfo(status: ConnectionStatus, attempt: number, max: number): { 
    label: string; 
    color: string;
    bgColor: string;
  } {
    switch (status) {
      case 'authenticated':
        return { label: 'Connected', color: 'text-emerald-600 dark:text-emerald-400', bgColor: 'bg-emerald-500' };
      case 'connected':
        return { label: 'Authenticating...', color: 'text-amber-600 dark:text-amber-400', bgColor: 'bg-amber-500' };
      case 'reconnecting':
        return { label: `Reconnecting (${attempt}/${max})`, color: 'text-amber-600 dark:text-amber-400', bgColor: 'bg-amber-500' };
      case 'connecting':
        return { label: 'Connecting...', color: 'text-amber-600 dark:text-amber-400', bgColor: 'bg-amber-500' };
      case 'disconnected':
        return { label: 'Disconnected', color: 'text-destructive', bgColor: 'bg-destructive' };
      default:
        return { label: 'Unknown', color: 'text-muted-foreground', bgColor: 'bg-muted' };
    }
  }

  function handleClick() {
    if ($state.status === 'disconnected') {
      socketClient.reconnect();
    }
  }

  const isLoading = $derived($state.status === 'reconnecting');
  const isDisconnected = $derived($state.status === 'disconnected');
  const isHealthy = $derived($state.status === 'authenticated');
  const statusInfo = $derived(getStatusInfo($state.status, $state.reconnectAttempt, $state.maxReconnectAttempts));
  
  // Only show when authenticated or there's a problem (disconnected/reconnecting)
  const shouldShow = $derived($state.status === 'authenticated' || $state.status === 'disconnected' || $state.status === 'reconnecting');
</script>

{#if shouldShow}
  {#if minimal}
    <!-- Minimal: just a dot with tooltip -->
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            type="button"
            class="inline-flex items-center justify-center p-1 rounded-full transition-colors
              {isDisconnected ? 'cursor-pointer hover:bg-accent' : 'cursor-default'}"
            onclick={handleClick}
            disabled={!isDisconnected}
            {...props}
          >
            <span 
              class="size-2.5 rounded-full {statusInfo.bgColor}
                {isLoading ? 'animate-pulse' : ''}"
            ></span>
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content side="bottom" class="text-xs">
        <p>{statusInfo.label}</p>
        {#if isDisconnected && !$state.error}
          <p class="text-muted-foreground">Click to reconnect</p>
        {/if}
        {#if $state.error}
          <p class="text-destructive">{$state.error}</p>
        {/if}
      </Tooltip.Content>
    </Tooltip.Root>
  {:else}
    <!-- Full: icon with dot indicator -->
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            type="button"
            class="relative inline-flex items-center justify-center size-8 rounded-md transition-colors
              {isDisconnected ? 'cursor-pointer hover:bg-accent' : 'cursor-default'}
              {statusInfo.color}"
            onclick={handleClick}
            disabled={!isDisconnected}
            {...props}
          >
            {#if isLoading}
              <LoaderIcon class="size-4 animate-spin" />
            {:else if isDisconnected}
              <WifiOffIcon class="size-4" />
            {:else}
              <WifiIcon class="size-4" />
            {/if}
            
            <!-- Status dot indicator -->
            <span 
              class="absolute -top-0.5 -right-0.5 size-2 rounded-full ring-2 ring-background {statusInfo.bgColor}
                {isLoading ? 'animate-pulse' : ''}"
            ></span>
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content side="bottom" class="text-xs">
        <p>{statusInfo.label}</p>
        {#if isDisconnected && !$state.error}
          <p class="text-muted-foreground">Click to reconnect</p>
        {/if}
        {#if $state.error}
          <p class="text-destructive">{$state.error}</p>
        {/if}
      </Tooltip.Content>
    </Tooltip.Root>
  {/if}
{/if}
