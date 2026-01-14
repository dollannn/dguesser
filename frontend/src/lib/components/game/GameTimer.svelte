<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    startedAt: number | null;
    durationMs: number;
    onTimeUp: () => void;
  }

  let { startedAt, durationMs, onTimeUp }: Props = $props();

  // Initialize with 0, will be set properly in effect/interval
  let remaining = $state(0);
  let intervalId: ReturnType<typeof setInterval>;
  let hasCalledTimeUp = $state(false);

  // Compute initial remaining when props change
  $effect(() => {
    if (startedAt !== null) {
      remaining = Math.max(0, durationMs - (Date.now() - startedAt));
    } else {
      remaining = durationMs;
    }
    hasCalledTimeUp = false;
  });

  let minutes = $derived(Math.floor(remaining / 60000));
  let seconds = $derived(Math.floor((remaining % 60000) / 1000));
  let isLow = $derived(remaining < 10000);
  let isExpired = $derived(remaining <= 0);

  onMount(() => {
    // Initialize remaining on mount
    if (startedAt !== null) {
      remaining = Math.max(0, durationMs - (Date.now() - startedAt));
    } else {
      remaining = durationMs;
    }

    intervalId = setInterval(() => {
      if (startedAt === null) return;

      remaining = Math.max(0, durationMs - (Date.now() - startedAt));

      if (remaining <= 0 && !hasCalledTimeUp) {
        hasCalledTimeUp = true;
        clearInterval(intervalId);
        onTimeUp();
      }
    }, 100);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });
</script>

<div
  class="font-mono text-2xl font-bold px-4 py-2 rounded-lg"
  class:bg-red-100={isLow}
  class:text-red-600={isLow}
  class:bg-gray-100={!isLow}
  class:text-gray-700={!isLow}
  class:animate-pulse={isLow && !isExpired}
>
  {minutes}:{seconds.toString().padStart(2, '0')}
</div>
