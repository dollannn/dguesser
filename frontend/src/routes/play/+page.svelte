<script lang="ts">
  import { goto } from '$app/navigation';
  import { user, authStore } from '$lib/stores/auth';
  import { gamesApi } from '$lib/api/games';
  import { partiesApi } from '$lib/api/parties';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import * as Card from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import { Separator } from '$lib/components/ui/separator';
  import * as Alert from '$lib/components/ui/alert';
  import SEO from '$lib/components/SEO.svelte';
  import PlayIcon from '@lucide/svelte/icons/play';
  import UsersIcon from '@lucide/svelte/icons/users';
  import TargetIcon from '@lucide/svelte/icons/target';
  import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import AlertCircleIcon from '@lucide/svelte/icons/alert-circle';
  import SparklesIcon from '@lucide/svelte/icons/sparkles';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import PartyPopperIcon from '@lucide/svelte/icons/party-popper';

  let joinCode = $state('');
  let loadingAction = $state<'solo' | 'multiplayer' | 'party' | 'join' | null>(null);
  let loading = $derived(loadingAction !== null);
  let error = $state('');

  async function startSoloGame() {
    if (loading) return;
    loadingAction = 'solo';
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'solo' });
      await goto(`/game/${game.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start game';
    } finally {
      loadingAction = null;
    }
  }

  async function createMultiplayerGame() {
    if (loading) return;
    loadingAction = 'multiplayer';
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'multiplayer' });
      await goto(`/game/${game.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create game';
    } finally {
      loadingAction = null;
    }
  }

  async function joinByCode() {
    if (!joinCode.trim() || loading) return;

    loadingAction = 'join';
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      // Unified lookup: checks parties first, then games
      const result = await partiesApi.joinByCode(joinCode.trim().toUpperCase());
      if (result.type === 'party') {
        await goto(`/party/${result.id}`);
      } else {
        await goto(`/game/${result.id}`);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Invalid code';
    } finally {
      loadingAction = null;
    }
  }

  async function createParty() {
    if (loading) return;
    loadingAction = 'party';
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const party = await partiesApi.create();
      await goto(`/party/${party.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create party';
    } finally {
      loadingAction = null;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      joinByCode();
    }
  }
</script>

<SEO
  title="Play"
  description="Start a new geography guessing game. Choose your map, difficulty, and challenge yourself to identify locations from around the world."
/>

<div class="min-h-[calc(100vh-4rem)] flex items-center justify-center p-4">
  <div class="w-full max-w-md space-y-6">
    <!-- Header -->
    <div class="text-center space-y-2">
      <Badge variant="secondary" class="mb-2">
        <SparklesIcon class="size-3 mr-1" />
        Free to play
      </Badge>
      <h1 class="text-3xl font-bold tracking-tight">Start Playing</h1>
      <p class="text-muted-foreground">
        Choose your game mode and jump right in
      </p>
    </div>

    <!-- Error Alert -->
    {#if error}
      <Alert.Root variant="destructive">
        <AlertCircleIcon class="size-4" />
        <Alert.Title>Error</Alert.Title>
        <Alert.Description>{error}</Alert.Description>
      </Alert.Root>
    {/if}

    <!-- Solo Play Card -->
    <Card.Root class="group hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
      <Card.Header class="pb-3">
        <div class="flex items-center gap-3">
          <div class="p-2 rounded-lg bg-primary/10 group-hover:bg-primary/20 transition-colors">
            <TargetIcon class="size-5 text-primary" />
          </div>
          <div>
            <Card.Title class="text-lg">Solo Play</Card.Title>
            <Card.Description>Practice on your own</Card.Description>
          </div>
        </div>
      </Card.Header>
      <Card.Content class="pb-3">
        <ul class="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          <li class="flex items-center gap-1">
            <MapPinIcon class="size-3 text-primary" />
            5 rounds
          </li>
          <li class="flex items-center gap-1">
            <TrophyIcon class="size-3 text-primary" />
            Track progress
          </li>
        </ul>
      </Card.Content>
      <Card.Footer>
        <Button
          onclick={startSoloGame}
          disabled={loading && loadingAction !== 'solo'}
          loading={loadingAction === 'solo'}
          class="w-full"
        >
          {#if loadingAction !== 'solo'}
            <PlayIcon class="size-4" />
          {/if}
          {loadingAction === 'solo' ? 'Starting...' : 'Play Solo'}
        </Button>
      </Card.Footer>
    </Card.Root>

    <!-- Multiplayer Card -->
    <Card.Root class="group hover:shadow-lg transition-all duration-300 border-2 hover:border-primary/50">
      <Card.Header class="pb-3">
        <div class="flex items-center gap-3">
          <div class="p-2 rounded-lg bg-primary/10 group-hover:bg-primary/20 transition-colors">
            <UsersIcon class="size-5 text-primary" />
          </div>
          <div>
            <Card.Title class="text-lg">Multiplayer</Card.Title>
            <Card.Description>Play with friends</Card.Description>
          </div>
        </div>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div class="grid grid-cols-2 gap-2">
          <Button
            variant="outline"
            onclick={createMultiplayerGame}
            disabled={loading && loadingAction !== 'multiplayer'}
            loading={loadingAction === 'multiplayer'}
            class="w-full"
          >
            {#if loadingAction !== 'multiplayer'}
              <UsersIcon class="size-4" />
            {/if}
            Quick Game
          </Button>
          <Button
            variant="outline"
            onclick={createParty}
            disabled={loading && loadingAction !== 'party'}
            loading={loadingAction === 'party'}
            class="w-full"
          >
            {#if loadingAction !== 'party'}
              <PartyPopperIcon class="size-4" />
            {/if}
            Create Party
          </Button>
        </div>

        <div class="relative">
          <div class="absolute inset-0 flex items-center">
            <Separator class="w-full" />
          </div>
          <div class="relative flex justify-center text-xs uppercase">
            <span class="bg-card px-2 text-muted-foreground">or join with code</span>
          </div>
        </div>

        <div class="flex gap-2">
          <Input
            type="text"
            bind:value={joinCode}
            placeholder="ABC123"
            maxlength={6}
            class="uppercase font-mono text-center tracking-widest"
            onkeydown={handleKeydown}
          />
          <Button
            variant="secondary"
            onclick={joinByCode}
            disabled={(loading && loadingAction !== 'join') || !joinCode.trim()}
            loading={loadingAction === 'join'}
          >
            {loadingAction === 'join' ? 'Joining...' : 'Join'}
            {#if loadingAction !== 'join'}
              <ArrowRightIcon class="size-4" />
            {/if}
          </Button>
        </div>
      </Card.Content>
    </Card.Root>

    <!-- Back link -->
    <div class="text-center">
      <Button variant="ghost" href="/" class="text-muted-foreground">
        <ArrowLeftIcon class="size-4" />
        Back to home
      </Button>
    </div>
  </div>
</div>
