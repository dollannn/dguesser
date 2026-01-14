<script lang="ts">
  interface Props {
    currentRound: number;
    totalRounds: number;
  }

  let { currentRound, totalRounds }: Props = $props();
</script>

<div class="flex items-center gap-2">
  <span class="text-lg font-semibold text-gray-700">
    Round {currentRound} of {totalRounds}
  </span>
</div>
