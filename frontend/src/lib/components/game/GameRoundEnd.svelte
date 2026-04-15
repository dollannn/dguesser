<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gamesApi } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore, formatDistance } from '$lib/utils.js';
  import { MARKER_CONFIG } from '$lib/config/map';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import ResultsMap from './ResultsMap.svelte';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import Loader2Icon from '@lucide/svelte/icons/loader-2';
  import SkipForwardIcon from '@lucide/svelte/icons/skip-forward';
  import VoteIcon from '@lucide/svelte/icons/hand';
  import CheckIcon from '@lucide/svelte/icons/check';
  import TimerIcon from '@lucide/svelte/icons/timer';

  interface Props {
    game: GameDetails;
    onNextRound?: () => void;
  }

  let { game, onNextRound }: Props = $props();

  let gameState = $derived($gameStore);
  let results = $derived(gameState.results);
  let correctLocation = $derived(gameState.location);
  
  // For solo mode: auto-transition to finished after last round
  let isLastRound = $derived(gameState.currentRound >= gameState.totalRounds);
  
  // Host detection for skip controls
  let isHost = $derived(gameState.hostId === $user?.id);
  let isMultiplayer = $derived(game.mode === 'multiplayer');
  
  // Between-rounds countdown (multiplayer)
  let countdownSeconds = $state<number | null>(null);
  
  // Countdown for auto-transition (only for last round in solo)
  let countdown = $state(3);
  let isTransitioning = $state(false);
  
  async function skipToResults() {
    if (isTransitioning) return;
    isTransitioning = true;
    
    try {
      // Fetch updated game data with final scores
      const updatedGame = await gamesApi.get(game.id);
      
      // Build final standings from players (sorted by score descending)
      const standings = updatedGame.players
        .toSorted((a, b) => b.score - a.score)
        .map((p, i) => ({
          rank: i + 1,
          user_id: p.user_id,
          display_name: p.display_name || 'You',
          total_score: p.score,
        }));
      
      // Trigger game end transition
      gameStore.handleGameEnd({
        game_id: game.id,
        final_standings: standings,
      });
    } catch (e) {
      console.error('Failed to fetch final results:', e);
      isTransitioning = false;
    }
  }
  
  // Solo mode: auto-transition to results after last round
  $effect(() => {
    if (game.mode === 'solo' && isLastRound) {
      countdown = 3;
      
      // Countdown timer
      const countdownInterval = setInterval(() => {
        countdown -= 1;
        if (countdown <= 0) {
          clearInterval(countdownInterval);
          skipToResults();
        }
      }, 1000);
      
      return () => clearInterval(countdownInterval);
    }
  });
  
  // Multiplayer: between-rounds countdown timer
  $effect(() => {
    const nextRoundAt = gameState.nextRoundAt;
    if (!nextRoundAt || !isMultiplayer) {
      countdownSeconds = null;
      return;
    }
    
    function updateCountdown() {
      const remaining = Math.max(0, Math.ceil((nextRoundAt! - Date.now()) / 1000));
      countdownSeconds = remaining;
    }
    
    updateCountdown();
    const timer = setInterval(updateCountdown, 250);
    
    return () => clearInterval(timer);
  });
  
  // Sort results by score (highest first) and assign ranks
  let rankedResults = $derived.by(() => {
    return results
      .toSorted((a, b) => b.score - a.score)
      .map((result, index) => ({
        ...result,
        rank: index + 1,
        isCurrentUser: result.user_id === $user?.id || (result.user_id === '' && game.mode === 'solo'),
      }));
  });

  // Build color map for each player (matches ResultsMap logic)
  // Current user always gets blue, others get cycling colors from players palette
  let playerColorMap = $derived.by(() => {
    const colorMap = new Map<string, string>();
    let otherIndex = 0;
    
    for (const result of rankedResults) {
      if (result.isCurrentUser) {
        colorMap.set(result.user_id, MARKER_CONFIG.colors.currentUser);
      } else {
        colorMap.set(
          result.user_id,
          MARKER_CONFIG.colors.players[otherIndex % MARKER_CONFIG.colors.players.length]
        );
        otherIndex++;
      }
    }
    
    return colorMap;
  });

  // Current user's ID for the map component
  let currentUserId = $derived($user?.id ?? '');
</script>

<div class="min-h-screen bg-background p-4 md:p-6 pt-20 md:pt-24">
  <div class="max-w-5xl mx-auto space-y-6">
    <!-- Header -->
    <div class="text-center space-y-2">
      <h2 class="text-2xl md:text-3xl font-bold tracking-tight">
        Round {gameState.currentRound} Complete
      </h2>
      <p class="text-muted-foreground">
        {gameState.currentRound} of {gameState.totalRounds} rounds
      </p>
    </div>

    <!-- Results map -->
    <Card.Root class="overflow-hidden p-0">
      <div class="h-[350px] md:h-[400px]">
        {#if correctLocation}
          <ResultsMap
            correctLat={correctLocation.lat}
            correctLng={correctLocation.lng}
            guesses={results
              .filter((r) => r.distance_meters >= 0)
              .map((r) => ({
                lat: r.guess_lat,
                lng: r.guess_lng,
                displayName: r.display_name,
                userId: r.user_id,
                distanceMeters: r.distance_meters,
              }))}
            {currentUserId}
          />
        {/if}
      </div>
    </Card.Root>

    <!-- Results table -->
    <Card.Root>
      <Card.Header class="pb-3">
        <Card.Title class="flex items-center gap-2">
          <TrophyIcon class="h-5 w-5 text-yellow-500" />
          Round Results
        </Card.Title>
      </Card.Header>
      <Card.Content class="p-0">
        <Table.Root>
          <Table.Header>
            <Table.Row class="hover:bg-transparent">
              <Table.Head class="w-16 pl-6">Rank</Table.Head>
              <Table.Head>Player</Table.Head>
              <Table.Head class="text-right">
                <span class="hidden sm:inline">Distance</span>
                <MapPinIcon class="inline sm:hidden h-4 w-4" />
              </Table.Head>
              <Table.Head class="text-right">Score</Table.Head>
              <Table.Head class="text-right pr-6 hidden sm:table-cell">Total</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each rankedResults as result (result.user_id)}
              {@const markerColor = playerColorMap.get(result.user_id)}
              <Table.Row 
                class={result.isCurrentUser ? 'bg-primary/5' : ''}
              >
                <Table.Cell class="pl-6 font-semibold">
                  <span class={getRankClass(result.rank)}>
                    {getRankDisplay(result.rank)}
                  </span>
                </Table.Cell>
                <Table.Cell>
                  <div class="flex items-center gap-2">
                    <!-- Color dot matching the map marker -->
                    {#if markerColor && result.distance_meters >= 0}
                      <span
                        class="inline-block w-3 h-3 rounded-full shrink-0 ring-2 ring-background"
                        style="background-color: {markerColor};"
                      ></span>
                    {/if}
                    <span class={result.isCurrentUser ? 'font-medium text-primary' : 'font-medium'}>
                      {result.display_name || 'You'}
                    </span>
                    {#if result.isCurrentUser}
                      <Badge variant="secondary" class="text-xs">You</Badge>
                    {/if}
                  </div>
                  <!-- Mobile total -->
                  <div class="sm:hidden text-xs text-muted-foreground mt-0.5 ml-5">
                    Total: {formatScore(result.total_score)}
                  </div>
                </Table.Cell>
                <Table.Cell class="text-right text-muted-foreground">
                  {formatDistance(result.distance_meters)}
                </Table.Cell>
                <Table.Cell class="text-right">
                  <span class="font-semibold text-green-600 dark:text-green-500">
                    +{formatScore(result.score)}
                  </span>
                </Table.Cell>
                <Table.Cell class="text-right pr-6 font-semibold hidden sm:table-cell">
                  {formatScore(result.total_score)}
                </Table.Cell>
              </Table.Row>
            {/each}
          </Table.Body>
        </Table.Root>
      </Card.Content>
    </Card.Root>

    <!-- Continue/Next action -->
    <div class="flex flex-col items-center gap-3 pt-2">
      {#if gameState.currentRound < gameState.totalRounds}
        {#if game.mode === 'solo' && onNextRound}
          <Button size="lg" onclick={onNextRound} class="gap-2">
            Continue to Round {gameState.currentRound + 1}
            <ArrowRightIcon class="h-5 w-5" />
          </Button>
        {:else}
          <!-- Multiplayer between-rounds: countdown + skip/vote controls -->
          <Card.Root class="w-full max-w-sm">
            <Card.Content class="flex flex-col items-center gap-4 py-5 px-6">
              <!-- Countdown timer -->
              {#if countdownSeconds !== null && countdownSeconds > 0}
                <div class="flex items-center gap-2 text-muted-foreground">
                  <TimerIcon class="h-5 w-5" />
                  <span class="font-medium">
                    Next round in <span class="text-foreground font-bold tabular-nums">{countdownSeconds}s</span>
                  </span>
                </div>
              {:else}
                <div class="flex items-center gap-2">
                  <Loader2Icon class="h-5 w-5 animate-spin text-primary" />
                  <span class="font-medium">Starting next round...</span>
                </div>
              {/if}

              <!-- Skip/Vote controls -->
              {#if isHost}
                <Button size="sm" variant="secondary" onclick={() => gameStore.skipWait()} class="gap-2">
                  <SkipForwardIcon class="h-4 w-4" />
                  Skip Wait
                </Button>
              {:else}
                <Button
                  size="sm"
                  variant="secondary"
                  onclick={() => gameStore.voteSkip()}
                  disabled={gameState.hasVotedToSkip}
                  class="gap-2"
                >
                  {#if gameState.hasVotedToSkip}
                    <CheckIcon class="h-4 w-4" />
                    Voted to Skip
                  {:else}
                    <VoteIcon class="h-4 w-4" />
                    Vote to Skip
                  {/if}
                </Button>
              {/if}

              <!-- Vote count -->
              {#if gameState.skipVotesRequired > 0}
                <p class="text-xs text-muted-foreground">
                  {gameState.skipVotes}/{gameState.skipVotesRequired} voted to skip
                </p>
              {/if}
            </Card.Content>
          </Card.Root>
        {/if}
      {:else}
        <!-- Last round - show skip to results button -->
        <Button size="lg" onclick={skipToResults} disabled={isTransitioning} class="gap-2">
          {#if isTransitioning}
            <Loader2Icon class="h-5 w-5 animate-spin" />
            Loading Results...
          {:else}
            <TrophyIcon class="h-5 w-5" />
            View Final Results
          {/if}
        </Button>
        {#if !isTransitioning && countdown > 0}
          <p class="text-sm text-muted-foreground">
            Auto-continuing in {countdown}s...
          </p>
        {/if}
      {/if}
    </div>
  </div>
</div>
