<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore } from '$lib/utils.js';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import HomeIcon from '@lucide/svelte/icons/home';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';

  interface Props {
    game: GameDetails;
  }

  let { game }: Props = $props();

  let standings = $derived($gameStore.finalStandings);
  let myStanding = $derived(standings.find((s) => s.user_id === $user?.id));
  let myRank = $derived(myStanding?.rank);
  let winner = $derived(standings[0]);
  let isWinner = $derived(myRank === 1);

  // Confetti effect for winner (shows for 4 seconds)
  let showConfetti = $state(true);
  $effect(() => {
    const timer = setTimeout(() => {
      showConfetti = false;
    }, 4000);
    return () => clearTimeout(timer);
  });

  // Confetti colors that match our design system
  const confettiColors = [
    'hsl(45, 93%, 47%)',   // Gold
    'hsl(220, 90%, 56%)',  // Blue
    'hsl(160, 84%, 39%)',  // Green
    'hsl(340, 82%, 52%)',  // Pink
    'hsl(280, 68%, 60%)',  // Purple
    'hsl(25, 95%, 53%)',   // Orange
  ];
</script>

<div class="min-h-screen bg-background p-4 md:p-6 pt-20 md:pt-24 relative overflow-hidden">
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
      <h2 class="text-2xl md:text-3xl font-bold tracking-tight">Game Complete</h2>
      <p class="text-muted-foreground">
        {game.total_rounds} rounds played
      </p>
    </div>

    <!-- Winner announcement -->
    {#if winner}
      <Card.Root class="overflow-hidden py-0">
        <div class="bg-gradient-to-br from-yellow-500/10 via-amber-500/5 to-orange-500/10 dark:from-yellow-500/20 dark:via-amber-500/10 dark:to-orange-500/20">
          <Card.Content class="py-5 text-center">
            <div class="inline-flex items-center justify-center w-14 h-14 rounded-full bg-yellow-500/20 mb-3">
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
              {@const isTop3 = standing.rank <= 3}
              <Table.Row class={isCurrentUser ? 'bg-primary/5' : ''}>
                <Table.Cell class="pl-6 font-semibold">
                  <span class={getRankClass(standing.rank)}>
                    {getRankDisplay(standing.rank)}
                  </span>
                </Table.Cell>
                <Table.Cell>
                  <div class="flex items-center gap-2">
                    <span class={isCurrentUser ? 'font-medium text-primary' : 'font-medium'}>
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
                  <span class="font-semibold">{formatScore(standing.total_score)}</span>
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
            <span class="font-bold text-primary">{getRankDisplay(myRank)}</span>
            place with
            <span class="font-semibold">{formatScore(myStanding.total_score)}</span>
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

    <!-- Action buttons -->
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
