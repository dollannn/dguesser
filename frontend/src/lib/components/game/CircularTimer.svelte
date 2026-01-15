<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    startedAt: number | null;
    durationMs: number;
    size?: number;
    strokeWidth?: number;
    onTimeUp?: () => void;
  }

  let {
    startedAt,
    durationMs,
    size = 64,
    strokeWidth = 4,
    onTimeUp,
  }: Props = $props();

  let remaining = $state(0);
  let intervalId: ReturnType<typeof setInterval>;
  let hasCalledTimeUp = $state(false);

  // SVG calculations - derived from props
  let radius = $derived((size - strokeWidth) / 2);
  let circumference = $derived(2 * Math.PI * radius);

  // Derived values
  let progress = $derived(Math.max(0, Math.min(1, remaining / durationMs)));
  let strokeDashoffset = $derived(circumference * (1 - progress));
  let minutes = $derived(Math.floor(remaining / 60000));
  let seconds = $derived(Math.floor((remaining % 60000) / 1000));
  let timeDisplay = $derived(`${minutes}:${seconds.toString().padStart(2, '0')}`);

  // Color based on remaining time
  let strokeColor = $derived(() => {
    if (progress > 0.5) return 'stroke-green-500';
    if (progress > 0.2) return 'stroke-yellow-500';
    return 'stroke-red-500';
  });

  let textColor = $derived(() => {
    if (progress > 0.5) return 'text-foreground';
    if (progress > 0.2) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-red-500';
  });

  let isLow = $derived(remaining < 10000 && remaining > 0);

  // Reset when props change
  $effect(() => {
    if (startedAt !== null) {
      remaining = Math.max(0, durationMs - (Date.now() - startedAt));
    } else {
      remaining = durationMs;
    }
    hasCalledTimeUp = false;
  });

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
        onTimeUp?.();
      }
    }, 50);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });
</script>

<div
  class="relative inline-flex items-center justify-center"
  class:animate-pulse={isLow}
  style="width: {size}px; height: {size}px;"
>
  <!-- Background circle -->
  <svg
    class="absolute transform -rotate-90"
    width={size}
    height={size}
    viewBox="0 0 {size} {size}"
  >
    <!-- Track -->
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke="currentColor"
      stroke-width={strokeWidth}
      class="text-muted/30"
    />
    <!-- Progress -->
    <circle
      cx={size / 2}
      cy={size / 2}
      r={radius}
      fill="none"
      stroke-width={strokeWidth}
      stroke-linecap="round"
      class="transition-all duration-100 {strokeColor()}"
      style="stroke-dasharray: {circumference}; stroke-dashoffset: {strokeDashoffset};"
    />
  </svg>

  <!-- Time display -->
  <span class="font-mono text-sm font-bold {textColor()} z-10">
    {timeDisplay}
  </span>
</div>
