<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    children?: Snippet;
    topLeft?: Snippet;
    topRight?: Snippet;
    bottomRight?: Snippet;
    center?: Snippet;
  }

  let { children, topLeft, topRight, bottomRight, center }: Props = $props();
</script>

<!-- HUD Overlay Container -->
<div class="absolute inset-0 pointer-events-none z-10">
  <!-- Top Left - Scoreboard -->
  {#if topLeft}
    <div class="absolute top-5 left-5 pointer-events-auto">
      {@render topLeft()}
    </div>
  {/if}

  <!-- Top Right - Round & Timer -->
  {#if topRight}
    <div class="absolute top-5 right-5 pointer-events-auto">
      {@render topRight()}
    </div>
  {/if}

  <!-- Bottom Right - Map -->
  {#if bottomRight}
    <div class="absolute bottom-6 right-6 pointer-events-auto">
      {@render bottomRight()}
    </div>
  {/if}

  <!-- Center - Status messages -->
  {#if center}
    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 pointer-events-auto">
      {@render center()}
    </div>
  {/if}

  <!-- Additional children -->
  {#if children}
    {@render children()}
  {/if}
</div>
