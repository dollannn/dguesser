<script lang="ts">
  import { user } from '$lib/stores/auth';
  import type {
    CompletedRoundInfo,
    GameDetails,
    GameResultsResponse,
    RoundResultInfo,
  } from '$lib/api/games';
  import { formatDistance, formatScore, getRankClass, getRankDisplay } from '$lib/utils.js';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import * as Card from '$lib/components/ui/card';
  import * as Table from '$lib/components/ui/table';
  import GameSummaryMap from './GameSummaryMap.svelte';
  import ResultsMap from './ResultsMap.svelte';
  import StreetView from './StreetView.svelte';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import CalendarIcon from '@lucide/svelte/icons/calendar';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import HomeIcon from '@lucide/svelte/icons/home';
  import MapIcon from '@lucide/svelte/icons/map';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import TargetIcon from '@lucide/svelte/icons/target';
  import TrophyIcon from '@lucide/svelte/icons/trophy';

  interface Props {
    game: GameDetails;
    results: GameResultsResponse;
  }

  type RankedRoundResult = RoundResultInfo & {
    rank: number;
    isCurrentUser: boolean;
  };

  let { game, results }: Props = $props();

  let selectedRoundIndex = $state(0);
  let selectedRound = $derived(results.rounds[selectedRoundIndex] ?? null);
  let currentUserId = $derived($user?.id ?? '');
  let isSolo = $derived(game.mode === 'solo');
  let isAbandoned = $derived(game.status === 'abandoned');
  let standings = $derived(results.final_standings);
  let winner = $derived(standings[0] ?? null);
  let myStanding = $derived.by(() => {
    const byUser = standings.find((standing) => standing.user_id === currentUserId);
    if (byUser) return byUser;
    return isSolo && standings.length === 1 ? (standings[0] ?? null) : null;
  });

  let roundLocations = $derived(results.rounds.map((round) => round.correct_location));
  let roundHistory = $derived(results.rounds.map((round) => round.results));
  let hasSummaryMapData = $derived(roundLocations.length > 0 && roundHistory.length > 0);

  function isCurrentUserResult(result: RoundResultInfo, round: CompletedRoundInfo): boolean {
    if (result.user_id === currentUserId) return true;
    return isSolo && round.results.length === 1;
  }

  function getMyRoundResult(round: CompletedRoundInfo): RoundResultInfo | null {
    const byUser = round.results.find((result) => result.user_id === currentUserId);
    if (byUser) return byUser;
    return isSolo && round.results.length === 1 ? (round.results[0] ?? null) : null;
  }

  let myRoundStats = $derived.by(() =>
    results.rounds
      .map((round) => {
        const myResult = getMyRoundResult(round);
        if (!myResult) return null;

        return {
          round: round.round_number,
          distance: myResult.distance_meters,
          score: myResult.score,
        };
      })
      .filter((stat): stat is NonNullable<typeof stat> => stat !== null)
  );

  let totalDistance = $derived.by(() =>
    myRoundStats
      .filter((stat) => stat.distance >= 0)
      .reduce((sum, stat) => sum + stat.distance, 0)
  );

  let averageDistance = $derived.by(() => {
    const guessedRounds = myRoundStats.filter((stat) => stat.distance >= 0);
    if (guessedRounds.length === 0) return 0;
    return totalDistance / guessedRounds.length;
  });

  let bestRound = $derived.by(() => {
    if (myRoundStats.length === 0) return null;
    return myRoundStats.reduce((best, stat) => (stat.score > best.score ? stat : best));
  });

  let rankedSelectedResults = $derived.by((): RankedRoundResult[] => {
    const round = selectedRound;
    if (!round) return [];

    return round.results
      .toSorted((a, b) => b.score - a.score || a.display_name.localeCompare(b.display_name))
      .map((result, index) => ({
        ...result,
        rank: index + 1,
        isCurrentUser: isCurrentUserResult(result, round),
      }));
  });

  let selectedMyResult = $derived.by(() => {
    if (!selectedRound) return null;
    return getMyRoundResult(selectedRound);
  });

  function formatDateTime(isoString: string | null): string {
    if (!isoString) return 'Unknown date';
    return new Date(isoString).toLocaleString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getModeLabel(mode: string): string {
    switch (mode) {
      case 'solo':
        return 'Solo';
      case 'multiplayer':
        return 'Multiplayer';
      case 'challenge':
        return 'Challenge';
      default:
        return mode;
    }
  }

  function formatTimeLimit(seconds: number): string {
    if (seconds === 0) return 'Unlimited';
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainder = seconds % 60;
    return remainder === 0 ? `${minutes}m` : `${minutes}m ${remainder}s`;
  }

  function getRestrictionLabel(): string {
    const enabled: string[] = [];
    if (game.settings.movement_allowed) enabled.push('Move');
    if (game.settings.zoom_allowed) enabled.push('Zoom');
    if (game.settings.rotation_allowed) enabled.push('Pan');
    return enabled.length > 0 ? enabled.join(' · ') : 'No movement aids';
  }

  function selectRound(index: number) {
    if (index < 0 || index >= results.rounds.length) return;
    selectedRoundIndex = index;
  }

  function selectRoundByNumber(roundNumber: number) {
    selectRound(results.rounds.findIndex((round) => round.round_number === roundNumber));
  }
</script>

<div class="min-h-screen bg-background px-4 py-6 md:py-10">
  <div class="mx-auto max-w-6xl space-y-6">
    <div class="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
      <div class="space-y-3">
        <Button variant="ghost" href="/history" class="w-fit gap-2 pl-0 text-muted-foreground">
          <ArrowLeftIcon class="h-4 w-4" />
          Back to History
        </Button>
        <div>
          <div class="mb-3 flex flex-wrap items-center gap-2">
            <Badge variant={isAbandoned ? 'destructive' : 'default'}>
              {isAbandoned ? 'Abandoned' : 'Historical Recap'}
            </Badge>
            <Badge variant="secondary">{getModeLabel(game.mode)}</Badge>
            <Badge variant="outline">{game.total_rounds} rounds</Badge>
          </div>
          <h1 class="text-3xl font-bold tracking-tight md:text-4xl">
            {isAbandoned ? 'Abandoned Game Review' : 'Game Recap'}
          </h1>
          <p class="mt-2 max-w-2xl text-muted-foreground">
            Review the final results, inspect every round, and look around the original Street View
            panoramas without changing the game state.
          </p>
        </div>
      </div>

      <div class="flex flex-wrap gap-2 md:pt-9">
        <Button variant="outline" href="/" class="gap-2">
          <HomeIcon class="h-4 w-4" />
          Home
        </Button>
        <Button href="/play" class="gap-2">
          <RotateCcwIcon class="h-4 w-4" />
          Play Again
        </Button>
      </div>
    </div>

    <div class="grid gap-4 md:grid-cols-4">
      <Card.Root class="md:col-span-2">
        <Card.Content class="flex items-center gap-4 py-6">
          <div class="rounded-xl bg-primary/10 p-3">
            <TrophyIcon class="h-7 w-7 text-primary" />
          </div>
          <div>
            <p class="text-sm text-muted-foreground">Your score</p>
            <p class="text-3xl font-bold">
              {formatScore(myStanding?.total_score ?? 0)}
            </p>
          </div>
          {#if myStanding}
            <Badge variant="secondary" class="ml-auto">
              {getRankDisplay(myStanding.rank)} place
            </Badge>
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Content class="py-6">
          <div class="flex items-center gap-2 text-muted-foreground">
            <TargetIcon class="h-4 w-4" />
            <span class="text-sm">Avg distance</span>
          </div>
          <p class="mt-2 text-2xl font-bold">{formatDistance(averageDistance)}</p>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Content class="py-6">
          <div class="flex items-center gap-2 text-muted-foreground">
            <CalendarIcon class="h-4 w-4" />
            <span class="text-sm">Played</span>
          </div>
          <p class="mt-2 text-sm font-medium leading-snug">
            {formatDateTime(game.ended_at ?? game.started_at ?? game.created_at)}
          </p>
        </Card.Content>
      </Card.Root>
    </div>

    <div class="grid gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
      <div class="space-y-6">
        {#if selectedRound}
          <Card.Root class="overflow-hidden">
            <Card.Header>
              <div class="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
                <div>
                  <Card.Title class="flex items-center gap-2">
                    <EyeIcon class="h-5 w-5 text-primary" />
                    Round {selectedRound.round_number} Street View
                  </Card.Title>
                  <Card.Description>
                    Look around the historical panorama. Movement follows the original game rules;
                    look and zoom are enabled for review.
                  </Card.Description>
                </div>
                {#if selectedMyResult}
                  <div class="rounded-lg bg-muted/60 px-4 py-2 text-sm">
                    <span class="text-muted-foreground">Your result:</span>
                    <span class="ml-1 font-semibold">{formatScore(selectedMyResult.score)}</span>
                    <span class="text-muted-foreground"> · {formatDistance(selectedMyResult.distance_meters)}</span>
                  </div>
                {/if}
              </div>
            </Card.Header>
            <Card.Content class="space-y-4">
              <div class="flex flex-wrap gap-2">
                {#each results.rounds as round, index (round.round_number)}
                  <Button
                    variant={index === selectedRoundIndex ? 'default' : 'outline'}
                    size="sm"
                    onclick={() => selectRound(index)}
                  >
                    Round {round.round_number}
                  </Button>
                {/each}
              </div>

              <div class="overflow-hidden rounded-xl border bg-gray-950">
                <div class="h-[360px] md:h-[520px]">
                  {#key `${game.id}-${selectedRound.round_number}`}
                    <StreetView
                      lat={selectedRound.correct_location.lat}
                      lng={selectedRound.correct_location.lng}
                      panoramaId={selectedRound.correct_location.panorama_id}
                      locationId={selectedRound.correct_location.location_id}
                      heading={selectedRound.correct_location.heading}
                      movementAllowed={game.settings.movement_allowed}
                      zoomAllowed={true}
                      rotationAllowed={true}
                      showReportButton={false}
                      autoReportOnNoCoverage={false}
                      fullScreen={false}
                    />
                  {/key}
                </div>
              </div>
            </Card.Content>
          </Card.Root>

          <Card.Root class="overflow-hidden">
            <Card.Header>
              <Card.Title class="flex items-center gap-2">
                <MapIcon class="h-5 w-5 text-primary" />
                Round {selectedRound.round_number} Results Map
              </Card.Title>
            </Card.Header>
            <Card.Content class="p-0">
              <div class="h-[340px] md:h-[420px]">
                <ResultsMap
                  correctLat={selectedRound.correct_location.lat}
                  correctLng={selectedRound.correct_location.lng}
                  guesses={rankedSelectedResults
                    .filter((result) => result.distance_meters >= 0)
                    .map((result) => ({
                      lat: result.guess_lat,
                      lng: result.guess_lng,
                      displayName: result.display_name,
                      userId: result.user_id,
                      distanceMeters: result.distance_meters,
                    }))}
                  {currentUserId}
                  animated={false}
                />
              </div>
            </Card.Content>
          </Card.Root>
        {:else}
          <Card.Root>
            <Card.Content class="py-12 text-center">
              <HistoryIcon class="mx-auto mb-4 h-12 w-12 text-muted-foreground/50" />
              <p class="text-lg font-medium">No completed rounds saved</p>
              <p class="mt-2 text-sm text-muted-foreground">
                This game ended before any round results were persisted.
              </p>
            </Card.Content>
          </Card.Root>
        {/if}
      </div>

      <aside class="space-y-6">
        <Card.Root>
          <Card.Header class="pb-3">
            <Card.Title class="flex items-center gap-2">
              <CrownIcon class="h-5 w-5 text-yellow-500" />
              Final Standings
            </Card.Title>
          </Card.Header>
          <Card.Content class="p-0">
            {#if standings.length > 0}
              <Table.Root>
                <Table.Body>
                  {#each standings as standing (standing.user_id)}
                    {@const isYou = standing.user_id === currentUserId || (isSolo && standings.length === 1)}
                    <Table.Row class={isYou ? 'bg-primary/5' : ''}>
                      <Table.Cell class="w-14 pl-6 font-semibold">
                        <span class={getRankClass(standing.rank)}>{getRankDisplay(standing.rank)}</span>
                      </Table.Cell>
                      <Table.Cell>
                        <div class="flex items-center gap-2">
                          <span class={isYou ? 'font-medium text-primary' : 'font-medium'}>
                            {standing.display_name}
                          </span>
                          {#if isYou}
                            <Badge variant="secondary" class="text-xs">You</Badge>
                          {/if}
                          {#if winner?.user_id === standing.user_id && !isAbandoned}
                            <TrophyIcon class="h-4 w-4 text-yellow-500" />
                          {/if}
                        </div>
                      </Table.Cell>
                      <Table.Cell class="pr-6 text-right font-semibold">
                        {formatScore(standing.total_score)}
                      </Table.Cell>
                    </Table.Row>
                  {/each}
                </Table.Body>
              </Table.Root>
            {:else}
              <div class="px-6 py-8 text-center text-sm text-muted-foreground">
                No standings were saved for this game.
              </div>
            {/if}
          </Card.Content>
        </Card.Root>

        <Card.Root>
          <Card.Header class="pb-3">
            <Card.Title class="flex items-center gap-2">
              <SettingsIcon class="h-5 w-5 text-primary" />
              Game Settings
            </Card.Title>
          </Card.Header>
          <Card.Content class="space-y-3 text-sm">
            <div class="flex justify-between gap-4">
              <span class="text-muted-foreground">Map</span>
              <span class="font-medium">{game.settings.map_id}</span>
            </div>
            <div class="flex justify-between gap-4">
              <span class="text-muted-foreground">Time limit</span>
              <span class="font-medium">{formatTimeLimit(game.settings.time_limit_seconds)}</span>
            </div>
            <div class="flex justify-between gap-4">
              <span class="text-muted-foreground">Controls</span>
              <span class="text-right font-medium">{getRestrictionLabel()}</span>
            </div>
            <div class="flex justify-between gap-4">
              <span class="text-muted-foreground">Game ID</span>
              <span class="font-mono text-xs text-muted-foreground">{game.id}</span>
            </div>
          </Card.Content>
        </Card.Root>

        {#if hasSummaryMapData}
          <Card.Root class="overflow-hidden">
            <Card.Header class="pb-3">
              <Card.Title class="flex items-center gap-2">
                <MapIcon class="h-5 w-5 text-primary" />
                Your Journey
              </Card.Title>
            </Card.Header>
            <Card.Content class="p-0">
              <div class="h-[320px]">
                <GameSummaryMap {roundLocations} {roundHistory} {currentUserId} />
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        {#if myRoundStats.length > 0}
          <Card.Root>
            <Card.Header class="pb-3">
              <Card.Title class="flex items-center gap-2">
                <TargetIcon class="h-5 w-5 text-primary" />
                Your Rounds
              </Card.Title>
            </Card.Header>
            <Card.Content class="p-0">
              <Table.Root>
                <Table.Header>
                  <Table.Row class="hover:bg-transparent">
                    <Table.Head class="pl-6">Round</Table.Head>
                    <Table.Head>Distance</Table.Head>
                    <Table.Head class="pr-6 text-right">Score</Table.Head>
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {#each myRoundStats as stat (stat.round)}
                    <Table.Row
                      class={selectedRound?.round_number === stat.round ? 'bg-primary/5' : 'cursor-pointer'}
                      onclick={() => selectRoundByNumber(stat.round)}
                      onkeydown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          selectRoundByNumber(stat.round);
                        }
                      }}
                      tabindex={0}
                      role="button"
                    >
                      <Table.Cell class="pl-6 font-medium">
                        <div class="flex items-center gap-2">
                          {stat.round}
                          {#if bestRound?.round === stat.round && myRoundStats.length > 1}
                            <Badge variant="secondary" class="text-xs">Best</Badge>
                          {/if}
                        </div>
                      </Table.Cell>
                      <Table.Cell class="text-muted-foreground">
                        {formatDistance(stat.distance)}
                      </Table.Cell>
                      <Table.Cell class="pr-6 text-right font-semibold">
                        {formatScore(stat.score)}
                      </Table.Cell>
                    </Table.Row>
                  {/each}
                </Table.Body>
              </Table.Root>
              <div class="border-t px-6 py-3 text-xs text-muted-foreground">
                Total distance: {formatDistance(totalDistance)}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}
      </aside>
    </div>
  </div>
</div>
