<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { partiesApi } from '$lib/api/parties';
  import type { PartyDetails } from '$lib/api/parties';
  import { partyStore } from '$lib/socket/party';
  import { user } from '$lib/stores/auth';
  import { authStore } from '$lib/stores/auth';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Badge } from '$lib/components/ui/badge';
  import * as Avatar from '$lib/components/ui/avatar';
  import { Separator } from '$lib/components/ui/separator';
  import GameSettingsForm from '$lib/components/game/GameSettingsForm.svelte';
  import { Spinner } from '$lib/components/ui/spinner';
  import { toast } from 'svelte-sonner';
  import SEO from '$lib/components/SEO.svelte';
  import PlayIcon from '@lucide/svelte/icons/play';
  import UsersIcon from '@lucide/svelte/icons/users';
  import CopyIcon from '@lucide/svelte/icons/copy';
  import CheckIcon from '@lucide/svelte/icons/check';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import XIcon from '@lucide/svelte/icons/x';

  let { data } = $props();
  const partyId = data.partyId;

  let loading = $state(true);
  let error = $state<string | null>(null);
  let party = $state<PartyDetails | null>(null);
  let copied = $state(false);
  let starting = $state(false);

  let partyState = $derived($partyStore);
  let isHost = $derived(partyState.hostId === $user?.id);
  let memberCount = $derived(partyState.members.size);
  let canStart = $derived(isHost && memberCount >= 2);
  let membersArray = $derived(Array.from(partyState.members.values()));

  onMount(async () => {
    try {
      // Ensure user session exists
      if (!$user) {
        await authStore.createGuest();
      }

      // Fetch party details via REST
      party = await partiesApi.get(partyId);

      // Connect via socket
      await partyStore.joinParty(partyId);
    } catch (e) {
      console.error('Failed to load party:', e);
      error = 'Party not found or has been disbanded.';
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    // Don't leave the party on unmount if we're navigating to a game
    // The party socket room persists alongside the game room
  });

  async function copyCode() {
    const code = partyState.joinCode || party?.join_code;
    if (!code) return;
    try {
      await navigator.clipboard.writeText(code);
      copied = true;
      toast.success('Code copied to clipboard!');
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      toast.error('Failed to copy code');
    }
  }

  async function handleStart() {
    if (starting) return;
    starting = true;
    try {
      partyStore.startGame();
    } catch {
      toast.error('Failed to start game');
    } finally {
      starting = false;
    }
  }

  function handleLeave() {
    partyStore.leaveParty();
    goto('/play');
  }

  function handleDisband() {
    partyStore.disbandParty();
  }

  function handleKick(userId: string) {
    partyStore.kickMember(userId);
  }

  function handleSettingsChange(settings: any) {
    partyStore.updateSettings(settings);
  }
</script>

<SEO
  title="Party Lobby - DGuesser"
  description="Party lobby - waiting for players"
/>

{#if loading}
  <div class="flex items-center justify-center h-64">
    <Spinner class="size-10 text-primary" />
  </div>
{:else if error}
  <div class="container mx-auto px-4 py-12 max-w-md text-center">
    <Card.Root>
      <Card.Content class="pt-6">
        <p class="text-muted-foreground">{error}</p>
        <Button href="/play" class="mt-4">Back to Play</Button>
      </Card.Content>
    </Card.Root>
  </div>
{:else if partyState.partyId}
  <div class="container mx-auto px-4 py-6 max-w-2xl space-y-6">
    <!-- Header -->
    <div class="text-center space-y-2">
      <h1 class="text-2xl font-bold">Party Lobby</h1>
      <p class="text-muted-foreground">
        Share the code below to invite friends
      </p>
    </div>

    <!-- Join Code -->
    <Card.Root>
      <Card.Content class="pt-6">
        <div class="flex items-center justify-center gap-3">
          <div
            class="text-3xl font-mono font-bold tracking-[0.3em] bg-muted px-6 py-3 rounded-lg select-all"
          >
            {(partyState.joinCode || '').slice(0, 3)}<span class="text-muted-foreground/50 mx-1"
              >-</span
            >{(partyState.joinCode || '').slice(3)}
          </div>
          <Button
            variant="outline"
            size="icon"
            onclick={copyCode}
          >
            {#if copied}
              <CheckIcon class="h-4 w-4 text-green-500" />
            {:else}
              <CopyIcon class="h-4 w-4" />
            {/if}
          </Button>
        </div>
      </Card.Content>
    </Card.Root>

    <!-- Members -->
    <Card.Root>
      <Card.Header>
        <Card.Title class="flex items-center gap-2">
          <UsersIcon class="h-5 w-5" />
          Players ({memberCount} / 8)
        </Card.Title>
      </Card.Header>
      <Card.Content>
        <div class="space-y-2">
          {#each membersArray as member (member.user_id)}
            <div
              class="flex items-center justify-between p-2 rounded-lg hover:bg-muted/50"
            >
              <div class="flex items-center gap-3">
                <Avatar.Root class="h-8 w-8">
                  {#if member.avatar_url}
                    <Avatar.Image
                      src={member.avatar_url}
                      alt={member.display_name}
                    />
                  {/if}
                  <Avatar.Fallback>
                    {member.display_name.charAt(0).toUpperCase()}
                  </Avatar.Fallback>
                </Avatar.Root>
                <span class="font-medium">{member.display_name}</span>
                {#if member.user_id === partyState.hostId}
                  <Badge variant="secondary" class="gap-1">
                    <CrownIcon class="h-3 w-3" />
                    Host
                  </Badge>
                {/if}
                {#if member.user_id === $user?.id}
                  <Badge variant="outline">You</Badge>
                {/if}
                {#if !member.connected}
                  <Badge variant="destructive" class="text-xs">
                    Disconnected
                  </Badge>
                {/if}
              </div>
              {#if isHost && member.user_id !== $user?.id}
                <Button
                  variant="ghost"
                  size="icon"
                  class="h-7 w-7 text-muted-foreground hover:text-destructive"
                  onclick={() => handleKick(member.user_id)}
                >
                  <XIcon class="h-4 w-4" />
                </Button>
              {/if}
            </div>
          {/each}
        </div>
      </Card.Content>
    </Card.Root>

    <!-- Settings -->
    <Card.Root>
      <Card.Header>
        <Card.Title>Game Settings</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if partyState.settings}
          <GameSettingsForm
            settings={partyState.settings}
            readonly={!isHost}
            onchange={handleSettingsChange}
          />
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- Actions -->
    <div class="flex flex-col gap-3">
      {#if isHost}
        <Button
          size="lg"
          class="w-full gap-2"
          disabled={!canStart || starting}
          onclick={handleStart}
        >
          <PlayIcon class="h-5 w-5" />
          {#if memberCount < 2}
            Need at least 2 players
          {:else if starting}
            Starting...
          {:else}
            Start Game
          {/if}
        </Button>
      {:else}
        <div class="text-center text-muted-foreground py-3">
          Waiting for host to start the game...
        </div>
      {/if}

      <Separator />

      <div class="flex gap-3">
        <Button
          variant="outline"
          class="flex-1 gap-2"
          onclick={handleLeave}
        >
          <LogOutIcon class="h-4 w-4" />
          Leave Party
        </Button>
        {#if isHost}
          <Button
            variant="destructive"
            class="gap-2"
            onclick={handleDisband}
          >
            <TrashIcon class="h-4 w-4" />
            Disband
          </Button>
        {/if}
      </div>
    </div>
  </div>
{/if}
