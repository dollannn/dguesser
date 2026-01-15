<script lang="ts">
  import { gamesApi, type GameDetails, type GameSummary } from '$lib/api/games';
  import { gameStore, type RoundResult } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore, formatDistance } from '$lib/utils.js';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import HomeIcon from '@lucide/svelte/icons/home';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import TargetIcon from '@lucide/svelte/icons/target';
  import TrendingUpIcon from '@lucide/svelte/icons/trending-up';
  import TrendingDownIcon from '@lucide/svelte/icons/trending-down';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import ChartLineIcon from '@lucide/svelte/icons/chart-line';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  // Game mode detection
  let isSolo = $derived(game.mode === 'solo');

  // Common derived values
  let standings = $derived($gameStore.finalStandings);
  let myStanding = $derived(standings.find((s) => s.user_id === $user?.id));
  let myRank = $derived(myStanding?.rank);
  let winner = $derived(standings[0]);
  let isWinner = $derived(myRank === 1);

  // Solo mode: round history and statistics
  let roundHistory = $derived($gameStore.roundHistory);
  let finalScore = $derived(myStanding?.total_score ?? winner?.total_score ?? 0);
  let previousBest = $derived($user?.best_score ?? 0);
  let isNewPersonalBest = $derived(finalScore > previousBest);

  // Extract user's results from round history (for solo, user_id might be empty string)
  interface RoundStat {
    round: number;
    distance: number;
    score: number;
  }

  let myRoundStats = $derived.by(() => {
    const stats: RoundStat[] = [];
    for (let i = 0; i < roundHistory.length; i++) {
      const roundResults: RoundResult[] = roundHistory[i];
      // In solo mode, there's only one result per round
      // user_id might be empty string from the REST API response
      const myResult = roundResults.find(
        (r: RoundResult) => r.user_id === $user?.id || r.user_id === ''
      );
      if (myResult) {
        stats.push({
          round: i + 1,
          distance: myResult.distance_meters,
          score: myResult.score,
        });
      }
    }
    return stats;
  });

  // Statistics calculations
  let avgDistance = $derived.by(() => {
    if (myRoundStats.length === 0) return 0;
    const validDistances = myRoundStats.filter((r) => r.distance >= 0);
    if (validDistances.length === 0) return 0;
    const total = validDistances.reduce((sum, r) => sum + r.distance, 0);
    return total / validDistances.length;
  });

  let totalDistance = $derived.by(() => {
    const validDistances = myRoundStats.filter((r) => r.distance >= 0);
    return validDistances.reduce((sum, r) => sum + r.distance, 0);
  });

  let bestRound = $derived.by(() => {
    if (myRoundStats.length === 0) return null;
    return myRoundStats.reduce((best, curr) =>
      curr.score > best.score ? curr : best
    );
  });

  let worstRound = $derived.by(() => {
    if (myRoundStats.length === 0) return null;
    return myRoundStats.reduce((worst, curr) =>
      curr.score < worst.score ? curr : worst
    );
  });

  // Recent games (async loaded for solo mode)
  let recentGames = $state<GameSummary[]>([]);
  let loadingHistory = $state(false);

  $effect(() => {
    if (isSolo) {
      loadingHistory = true;
      gamesApi
        .getHistory()
        .then((games) => {
          // Filter to solo games only, exclude current game
          recentGames = games
            .filter((g) => g.mode === 'solo' && g.id !== game.id)
            .slice(0, 5);
        })
        .catch((e) => {
          console.error('Failed to load game history:', e);
        })
        .finally(() => {
          loadingHistory = false;
        });
    }
  });

  // Confetti effect - for multiplayer: always on win, for solo: only on personal best
  let showConfetti = $state(false);

  $effect(() => {
    if (isSolo) {
      showConfetti = isNewPersonalBest;
    } else {
      showConfetti = isWinner;
    }

    // Auto-hide after 4 seconds
    if (showConfetti) {
      const timer = setTimeout(() => {
        showConfetti = false;
      }, 4000);
      return () => clearTimeout(timer);
    }
  });

  // Confetti colors that match our design system
  const confettiColors = [
    'hsl(45, 93%, 47%)', // Gold
    'hsl(220, 90%, 56%)', // Blue
    'hsl(160, 84%, 39%)', // Green
    'hsl(340, 82%, 52%)', // Pink
    'hsl(280, 68%, 60%)', // Purple
    'hsl(25, 95%, 53%)', // Orange
  ];

  // Format relative date for recent games
  function formatRelativeDate(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'Today';
    if (diffDays === 1) return 'Yesterday';
    if (diffDays < 7) return `${diffDays} days ago`;
    if (diffDays < 30) return `${Math.floor(diffDays / 7)} weeks ago`;
    return date.toLocaleDateString();
  }
</script>

<div
  class="min-h-screen bg-background p-4 md:p-6 pt-20 md:pt-24 relative overflow-hidden"
>
  <!-- Confetti animation -->
  {#if showConfetti}
    <div class="absolute inset-0 pointer-events-none overflow-hidden z-0">
      {#each Array(24) as _, i}
        <div
          class="confetti-piece absolute w-2.5 h-2.5 rounded-sm opacity-90"
          style:left="{Math.random() * 100}%"
          style:animation-duration="{2.5 + Math.random() * 2}s"
          style:animation-delay="{Math.random() * 0.8}s"
          style:background={confettiColors[i % confettiColors.length]}
        ></div>
      {/each}
    </div>
  {/if}

  <div class="max-w-2xl mx-auto space-y-6 relative z-10">
    <!-- Header -->
    <div class="text-center space-y-2">
      <h2 class="text-2xl md:text-3xl font-bold tracking-tight">
        Game Complete
      </h2>
      <p class="text-muted-foreground">
        {game.total_rounds} rounds played
      </p>
    </div>

    {#if isSolo}
      <!-- ==================== SOLO MODE UI ==================== -->

      <!-- Score Hero Section -->
      <Card.Root class="overflow-hidden py-0">
        <div
          class={isNewPersonalBest
            ? 'bg-gradient-to-br from-yellow-500/10 via-amber-500/5 to-orange-500/10 dark:from-yellow-500/20 dark:via-amber-500/10 dark:to-orange-500/20'
            : 'bg-gradient-to-br from-primary/5 via-primary/2 to-primary/5'}
        >
          <Card.Content class="py-8 text-center">
            {#if isNewPersonalBest}
              <div
                class="inline-flex items-center justify-center w-14 h-14 rounded-full bg-yellow-500/20 mb-3"
              >
                <SparklesIcon class="h-7 w-7 text-yellow-500" />
              </div>
            {:else}
              <div
                class="inline-flex items-center justify-center w-14 h-14 rounded-full bg-primary/10 mb-3"
              >
                <TargetIcon class="h-7 w-7 text-primary" />
              </div>
            {/if}

            <div class="text-5xl md:text-6xl font-bold mb-2">
              {formatScore(finalScore)}
            </div>
            <p class="text-muted-foreground mb-4">points</p>

            {#if isNewPersonalBest}
              <Badge
                class="bg-yellow-500 hover:bg-yellow-500 text-yellow-950 text-sm px-4 py-1.5"
              >
                <SparklesIcon class="h-4 w-4 mr-1.5" />
                New Personal Best!
              </Badge>
            {:else if previousBest > 0}
              {@const diff = finalScore - previousBest}
              <div class="space-y-1">
                <p class="text-sm text-muted-foreground">
                  Personal Best: <span class="font-semibold"
                    >{formatScore(previousBest)}</span
                  >
                </p>
                {#if diff >= 0}
                  <p class="text-sm text-green-600 dark:text-green-400">
                    <TrendingUpIcon class="h-4 w-4 inline mr-1" />
                    Matched your best!
                  </p>
                {:else}
                  <p class="text-sm text-muted-foreground">
                    {formatScore(Math.abs(diff))} points from your best
                  </p>
                {/if}
              </div>
            {:else}
              <p class="text-sm text-muted-foreground">
                This is your first completed game!
              </p>
            {/if}
          </Card.Content>
        </div>
      </Card.Root>

      <!-- Round-by-Round Breakdown -->
      {#if myRoundStats.length > 0}
        <Card.Root>
          <Card.Header class="pb-3">
            <Card.Title class="flex items-center gap-2">
              <ChartLineIcon class="h-5 w-5 text-primary" />
              Round Breakdown
            </Card.Title>
          </Card.Header>
          <Card.Content class="p-0">
            <Table.Root>
              <Table.Header>
                <Table.Row class="hover:bg-transparent">
                  <Table.Head class="w-20 pl-6">Round</Table.Head>
                  <Table.Head>Distance</Table.Head>
                  <Table.Head class="text-right pr-6">Score</Table.Head>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {#each myRoundStats as stat (stat.round)}
                  {@const isBest =
                    bestRound && stat.round === bestRound.round && myRoundStats.length > 1}
                  {@const isWorst =
                    worstRound && stat.round === worstRound.round && myRoundStats.length > 1}
                  <Table.Row
                    class={isBest
                      ? 'bg-green-500/5'
                      : isWorst
                        ? 'bg-red-500/5'
                        : ''}
                  >
                    <Table.Cell class="pl-6 font-medium">
                      {stat.round}
                    </Table.Cell>
                    <Table.Cell>
                      <span class="text-muted-foreground">
                        {formatDistance(stat.distance)}
                      </span>
                    </Table.Cell>
                    <Table.Cell class="text-right pr-6">
                      <div class="flex items-center justify-end gap-2">
                        <span class="font-semibold"
                          >{formatScore(stat.score)}</span
                        >
                        {#if isBest}
                          <Badge
                            variant="secondary"
                            class="text-xs bg-green-500/10 text-green-600 dark:text-green-400"
                          >
                            <TrendingUpIcon class="h-3 w-3 mr-1" />
                            Best
                          </Badge>
                        {:else if isWorst}
                          <Badge
                            variant="secondary"
                            class="text-xs bg-red-500/10 text-red-600 dark:text-red-400"
                          >
                            <TrendingDownIcon class="h-3 w-3 mr-1" />
                            Worst
                          </Badge>
                        {/if}
                      </div>
                    </Table.Cell>
                  </Table.Row>
                {/each}
              </Table.Body>
            </Table.Root>
          </Card.Content>
        </Card.Root>
      {/if}

      <!-- Game Statistics -->
      {#if myRoundStats.length > 0}
        <Card.Root>
          <Card.Header class="pb-3">
            <Card.Title class="flex items-center gap-2">
              <TargetIcon class="h-5 w-5 text-primary" />
              Statistics
            </Card.Title>
          </Card.Header>
          <Card.Content>
            <div class="grid grid-cols-2 gap-4">
              <div class="text-center p-3 rounded-lg bg-muted/50">
                <p class="text-2xl font-bold">{formatDistance(avgDistance)}</p>
                <p class="text-sm text-muted-foreground">Avg Distance</p>
              </div>
              <div class="text-center p-3 rounded-lg bg-muted/50">
                <p class="text-2xl font-bold">{formatDistance(totalDistance)}</p>
                <p class="text-sm text-muted-foreground">Total Distance</p>
              </div>
              {#if bestRound}
                <div class="text-center p-3 rounded-lg bg-green-500/5">
                  <p class="text-2xl font-bold text-green-600 dark:text-green-400">
                    {formatScore(bestRound.score)}
                  </p>
                  <p class="text-sm text-muted-foreground">
                    Best (Round {bestRound.round})
                  </p>
                </div>
              {/if}
              {#if worstRound}
                <div class="text-center p-3 rounded-lg bg-red-500/5">
                  <p class="text-2xl font-bold text-red-600 dark:text-red-400">
                    {formatScore(worstRound.score)}
                  </p>
                  <p class="text-sm text-muted-foreground">
                    Worst (Round {worstRound.round})
                  </p>
                </div>
              {/if}
            </div>
          </Card.Content>
        </Card.Root>
      {/if}

      <!-- Recent Games Comparison -->
      {#if recentGames.length > 0}
        <Card.Root>
          <Card.Header class="pb-3">
            <Card.Title class="flex items-center gap-2">
              <HistoryIcon class="h-5 w-5 text-primary" />
              Recent Games
            </Card.Title>
          </Card.Header>
          <Card.Content class="p-0">
            <div class="divide-y">
              {#each recentGames as pastGame (pastGame.id)}
                {@const diff = finalScore - pastGame.score}
                <div class="flex items-center justify-between px-6 py-3">
                  <span class="text-sm text-muted-foreground">
                    {formatRelativeDate(pastGame.played_at)}
                  </span>
                  <div class="flex items-center gap-3">
                    <span class="font-medium"
                      >{formatScore(pastGame.score)}</span
                    >
                    {#if diff > 0}
                      <Badge
                        variant="secondary"
                        class="text-xs bg-green-500/10 text-green-600 dark:text-green-400"
                      >
                        +{formatScore(diff)}
                      </Badge>
                    {:else if diff < 0}
                      <Badge
                        variant="secondary"
                        class="text-xs bg-red-500/10 text-red-600 dark:text-red-400"
                      >
                        {formatScore(diff)}
                      </Badge>
                    {:else}
                      <Badge variant="secondary" class="text-xs">=</Badge>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </Card.Content>
        </Card.Root>
      {/if}
    {:else}
      <!-- ==================== MULTIPLAYER MODE UI ==================== -->

      <!-- Winner announcement -->
      {#if winner}
        <Card.Root class="overflow-hidden py-0">
          <div
            class="bg-gradient-to-br from-yellow-500/10 via-amber-500/5 to-orange-500/10 dark:from-yellow-500/20 dark:via-amber-500/10 dark:to-orange-500/20"
          >
            <Card.Content class="py-5 text-center">
              <div
                class="inline-flex items-center justify-center w-14 h-14 rounded-full bg-yellow-500/20 mb-3"
              >
                <TrophyIcon class="h-7 w-7 text-yellow-500" />
              </div>
              <h1 class="text-2xl md:text-3xl font-bold mb-2">
                {winner.display_name} Wins!
              </h1>
              <Badge variant="secondary" class="text-base px-4 py-1">
                {formatScore(winner.total_score)} points
              </Badge>
            </Card.Content>
          </div>
        </Card.Root>
      {/if}

      <!-- Final standings table -->
      <Card.Root>
        <Card.Header class="pb-3">
          <Card.Title class="flex items-center gap-2">
            <CrownIcon class="h-5 w-5 text-yellow-500" />
            Final Standings
          </Card.Title>
        </Card.Header>
        <Card.Content class="p-0">
          <Table.Root>
            <Table.Header>
              <Table.Row class="hover:bg-transparent">
                <Table.Head class="w-16 pl-6">Rank</Table.Head>
                <Table.Head>Player</Table.Head>
                <Table.Head class="text-right pr-6">Score</Table.Head>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {#each standings as standing (standing.user_id)}
                {@const isCurrentUser = standing.user_id === $user?.id}
                <Table.Row class={isCurrentUser ? 'bg-primary/5' : ''}>
                  <Table.Cell class="pl-6 font-semibold">
                    <span class={getRankClass(standing.rank)}>
                      {getRankDisplay(standing.rank)}
                    </span>
                  </Table.Cell>
                  <Table.Cell>
                    <div class="flex items-center gap-2">
                      <span
                        class={isCurrentUser
                          ? 'font-medium text-primary'
                          : 'font-medium'}
                      >
                        {standing.display_name}
                      </span>
                      {#if isCurrentUser}
                        <Badge variant="secondary" class="text-xs">You</Badge>
                      {/if}
                      {#if standing.rank === 1}
                        <TrophyIcon class="h-4 w-4 text-yellow-500" />
                      {/if}
                    </div>
                  </Table.Cell>
                  <Table.Cell class="text-right pr-6">
                    <span class="font-semibold"
                      >{formatScore(standing.total_score)}</span
                    >
                  </Table.Cell>
                </Table.Row>
              {/each}
            </Table.Body>
          </Table.Root>
        </Card.Content>
      </Card.Root>

      <!-- Your result highlight -->
      {#if myRank && myRank > 1 && myStanding}
        <Card.Root class="border-primary/20 bg-primary/5">
          <Card.Content class="py-5 text-center">
            <p class="text-lg">
              You finished in
              <span class="font-bold text-primary"
                >{getRankDisplay(myRank)}</span
              >
              place with
              <span class="font-semibold"
                >{formatScore(myStanding.total_score)}</span
              >
              points
            </p>
          </Card.Content>
        </Card.Root>
      {:else if isWinner}
        <Card.Root class="border-yellow-500/30 bg-yellow-500/10">
          <Card.Content class="py-5 text-center">
            <p class="text-lg font-semibold text-yellow-700 dark:text-yellow-400">
              Congratulations! You're the champion!
            </p>
          </Card.Content>
        </Card.Root>
      {/if}
    {/if}

    <!-- Action buttons (shared) -->
    <div class="flex justify-center gap-3 pt-2">
      <Button variant="outline" href="/" class="gap-2">
        <HomeIcon class="h-4 w-4" />
        Back to Home
      </Button>
      <Button href="/play" class="gap-2">
        <RotateCcwIcon class="h-4 w-4" />
        Play Again
      </Button>
    </div>
  </div>
</div>

<style>
  @keyframes -global-confetti-fall {
    0% {
      transform: translateY(-20px) rotate(0deg);
      opacity: 1;
    }
    100% {
      transform: translateY(100vh) rotate(720deg);
      opacity: 0;
    }
  }

  .confetti-piece {
    animation-name: confetti-fall;
    animation-timing-function: ease-out;
    animation-fill-mode: forwards;
  }
</style>
