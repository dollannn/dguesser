<script lang="ts">
  import type { GameDetails, GameSettings } from '$lib/api/games';
  import { gamesApi } from '$lib/api/games';
  import { user } from '$lib/stores/auth';
  import { gameStore } from '$lib/socket/game';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import * as Avatar from '$lib/components/ui/avatar';
  import { Separator } from '$lib/components/ui/separator';
  import GameSettingsForm from './GameSettingsForm.svelte';
  import PlayIcon from '@lucide/svelte/icons/play';
  import UsersIcon from '@lucide/svelte/icons/users';
  import TargetIcon from '@lucide/svelte/icons/target';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import { toast } from 'svelte-sonner';

  interface Props {
    game: GameDetails;
    onStart: () => void;
  }

  let { game, onStart }: Props = $props();

  let gameState = $derived($gameStore);
  // Check both API response (game.players) and live socket state ($gameStore.players)
  // Socket state is updated when player joins/leaves
  let isPlayerFromApi = $derived(game.players.some((p) => p.user_id === $user?.id));
  let isPlayerFromSocket = $derived($user?.id ? $gameStore.players.has($user.id) : false);
  let isPlayer = $derived(isPlayerFromApi || isPlayerFromSocket);
  let isHost = $derived(game.players.find((p) => p.user_id === $user?.id)?.is_host ?? false);
  // Use socket state for player count if available, otherwise fall back to API
  let playerCount = $derived($gameStore.players.size > 0 ? $gameStore.players.size : game.players.length);
  let canStart = $derived(isHost && (game.mode === 'solo' || playerCount >= 2));
  let canJoin = $derived(!isPlayer && game.mode === 'multiplayer' && playerCount < 8);

  function joinGame() {
    gameStore.joinGame(game.id);
  }
  
  let copied = $state(false);
  let joinCode = $derived(game.join_code ?? '------');

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(joinCode);
      copied = true;
      toast.success('Code copied to clipboard!');
      setTimeout(() => (copied = false), 2000);
    } catch {
      toast.error('Failed to copy code');
    }
  }

  function getInitials(name: string): string {
    return name
      .split(' ')
      .map((n) => n[0])
      .join('')
      .toUpperCase()
      .slice(0, 2);
  }

  // Use socket state settings if available, otherwise fall back to API
  let currentSettings = $derived($gameStore.settings ?? game.settings);

  // Handle settings changes from the form
  let isUpdatingSettings = $state(false);
  async function handleSettingsChange(changes: Partial<GameSettings>) {
    if (isUpdatingSettings || !isHost) return;
    
    isUpdatingSettings = true;
    try {
      await gamesApi.updateSettings(game.id, changes);
      // Socket will broadcast the update to all players
    } catch (error) {
      console.error('Failed to update settings:', error);
      toast.error('Failed to update settings');
    } finally {
      isUpdatingSettings = false;
    }
  }
</script>

<div class="min-h-[calc(100vh-4rem)] flex items-center justify-center p-4">
  <Card.Root class="w-full max-w-lg">
    <Card.Header class="text-center pb-2">
      <div class="flex items-center justify-center gap-2 mb-2">
        {#if game.mode === 'solo'}
          <div class="p-2 rounded-lg bg-primary/10">
            <TargetIcon class="size-6 text-primary" />
          </div>
        {:else}
          <div class="p-2 rounded-lg bg-primary/10">
            <UsersIcon class="size-6 text-primary" />
          </div>
        {/if}
      </div>
      <Card.Title class="text-2xl">
        {game.mode === 'solo' ? 'Solo Game' : 'Multiplayer Lobby'}
      </Card.Title>
      <Card.Description>
        {#if game.mode === 'solo'}
          Ready when you are
        {:else if playerCount < 2}
          Waiting for more players to join...
        {:else}
          {playerCount} players ready
        {/if}
      </Card.Description>
    </Card.Header>

    <Card.Content class="space-y-6">
      <!-- Join Code (Multiplayer only) -->
      {#if game.mode === 'multiplayer'}
        <div class="rounded-lg border bg-muted/50 p-4 text-center">
          <p class="text-sm text-muted-foreground mb-2">Share this code with friends</p>
          <div class="flex items-center justify-center gap-2">
            <span class="text-3xl font-mono font-bold tracking-[0.3em] text-primary">
              {joinCode}
            </span>
            <Button
              variant="ghost"
              size="icon"
              onclick={copyCode}
              class="shrink-0"
              disabled={game.status === 'waiting'}
            >
              {#if copied}
                <CheckIcon class="size-4 text-green-500" />
              {:else}
                <CopyIcon class="size-4" />
              {/if}
            </Button>
          </div>
        </div>
      {/if}

      <!-- Game Settings -->
      <div>
        <div class="flex items-center gap-2 mb-3">
          <SettingsIcon class="size-4 text-muted-foreground" />
          <h3 class="text-sm font-medium text-muted-foreground">Game Settings</h3>
          {#if isHost}
            <Badge variant="outline" class="text-xs ml-auto">Editing</Badge>
          {/if}
        </div>
        <GameSettingsForm 
          settings={currentSettings}
          readonly={!isHost}
          onchange={handleSettingsChange}
        />
      </div>

      <!-- Players List (Multiplayer only) -->
      {#if game.mode === 'multiplayer'}
        <div>
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-medium text-muted-foreground">Players</h3>
            <Badge variant="secondary">{playerCount} / 8</Badge>
          </div>
          <div class="space-y-2">
            {#each game.players as player}
              {@const isYou = player.user_id === $user?.id}
              <div
                class="flex items-center justify-between rounded-lg border bg-card p-3 transition-colors hover:bg-muted/50 {isYou ? 'ring-2 ring-primary/50' : ''}"
              >
                <div class="flex items-center gap-3">
                  <Avatar.Root class="size-8">
                    {#if player.avatar_url}
                      <Avatar.Image src={player.avatar_url} alt={player.display_name} />
                    {/if}
                    <Avatar.Fallback class="text-xs">
                      {getInitials(player.display_name)}
                    </Avatar.Fallback>
                  </Avatar.Root>
                  <div class="flex items-center gap-2">
                    <span class="font-medium">{player.display_name}</span>
                    {#if isYou}
                      <Badge variant="outline" class="text-xs py-0">You</Badge>
                    {/if}
                  </div>
                </div>
                <div class="flex items-center gap-2">
                  {#if player.is_guest}
                    <Badge variant="secondary" class="text-xs">Guest</Badge>
                  {/if}
                  {#if player.is_host}
                    <Badge variant="default" class="gap-1">
                      <CrownIcon class="size-3" />
                      Host
                    </Badge>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </Card.Content>

    <Separator />

    <Card.Footer class="pt-6">
      <div class="w-full text-center">
        {#if canJoin}
          <Button onclick={joinGame} size="lg" class="w-full sm:w-auto px-8">
            <UsersIcon class="size-5" />
            Join Game
          </Button>
        {:else if canStart}
          <Button onclick={onStart} size="lg" class="w-full sm:w-auto px-8">
            <PlayIcon class="size-5" />
            Start Game
          </Button>
        {:else if !isHost}
          <p class="text-muted-foreground">Waiting for host to start the game...</p>
        {:else}
          <p class="text-muted-foreground">Need at least 2 players to start</p>
        {/if}
      </div>
    </Card.Footer>
  </Card.Root>
</div>
