<script lang="ts">
  import { MapPin } from '@lucide/svelte';

  interface Props {
    currentRound: number;
    totalRounds: number;
  }

  let { currentRound, totalRounds }: Props = $props();
</script>

<div
  class="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-background/85 backdrop-blur-md border border-border/50 shadow-lg"
>
  <MapPin class="w-4 h-4 text-primary" />
  <span class="text-sm font-semibold text-foreground">
    Round <span class="text-primary">{currentRound}</span>
    <span class="text-muted-foreground">/ {totalRounds}</span>
  </span>
</div>
