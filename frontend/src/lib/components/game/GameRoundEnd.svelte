<script lang="ts">
  import type { GameDetails } from '$lib/api/games';
  import { gamesApi } from '$lib/api/games';
  import { gameStore } from '$lib/socket/game';
  import { user } from '$lib/stores/auth';
  import { getRankDisplay, getRankClass, formatScore, formatDistance } from '$lib/utils.js';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import ResultsMap from './ResultsMap.svelte';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import Loader2Icon from '@lucide/svelte/icons/loader-2';

  interface Props {
    game: GameDetails;
    onNextRound?: () => void;
  }

  let { game, onNextRound }: Props = $props();

  let state = $derived($gameStore);
  let results = $derived(state.results);
  let correctLocation = $derived(state.location);
  
  // For solo mode: auto-transition to finished after last round
  let isLastRound = $derived(state.currentRound >= state.totalRounds);
  
  $effect(() => {
    if (game.mode === 'solo' && isLastRound) {
      const timer = setTimeout(async () => {
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
        }
      }, 3000);
      
      return () => clearTimeout(timer);
    }
  });
  
  // Sort results by score (highest first) and assign ranks
  let rankedResults = $derived(() => {
    return results
      .toSorted((a, b) => b.score - a.score)
      .map((result, index) => ({
        ...result,
        rank: index + 1,
        isCurrentUser: result.user_id === $user?.id,
      }));
  });
</script>

<div class="min-h-screen bg-background p-4 md:p-6 pt-20 md:pt-24">
  <div class="max-w-5xl mx-auto space-y-6">
    <!-- Header -->
    <div class="text-center space-y-2">
      <h2 class="text-2xl md:text-3xl font-bold tracking-tight">
        Round {state.currentRound} Complete
      </h2>
      <p class="text-muted-foreground">
        {state.currentRound} of {state.totalRounds} rounds
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
              }))}
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
            {#each rankedResults() as result (result.user_id)}
              {@const isTop3 = result.rank <= 3}
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
                    <span class={result.isCurrentUser ? 'font-medium text-primary' : 'font-medium'}>
                      {result.display_name || 'You'}
                    </span>
                    {#if result.isCurrentUser}
                      <Badge variant="secondary" class="text-xs">You</Badge>
                    {/if}
                  </div>
                  <!-- Mobile total -->
                  <div class="sm:hidden text-xs text-muted-foreground mt-0.5">
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
    <div class="flex justify-center pt-2">
      {#if state.currentRound < state.totalRounds}
        {#if game.mode === 'solo' && onNextRound}
          <Button size="lg" onclick={onNextRound} class="gap-2">
            Continue to Round {state.currentRound + 1}
            <ArrowRightIcon class="h-5 w-5" />
          </Button>
        {:else}
          <Card.Root class="inline-flex">
            <Card.Content class="flex items-center gap-3 py-3 px-5">
              <Loader2Icon class="h-5 w-5 animate-spin text-primary" />
              <span class="font-medium">Next round starting soon...</span>
            </Card.Content>
          </Card.Root>
        {/if}
      {:else}
        <Card.Root class="inline-flex border-primary/20 bg-primary/5">
          <Card.Content class="flex items-center gap-3 py-3 px-5">
            <TrophyIcon class="h-5 w-5 text-primary" />
            <span class="font-medium text-primary">Final results coming up...</span>
          </Card.Content>
        </Card.Root>
      {/if}
    </div>
  </div>
</div>
