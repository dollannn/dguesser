<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { partyStore } from '$lib/socket/party';
  import { socketClient } from '$lib/socket/client';
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
  import LinkIcon from '@lucide/svelte/icons/link';
  import CrownIcon from '@lucide/svelte/icons/crown';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import XIcon from '@lucide/svelte/icons/x';

  let { data } = $props();
  let partyId = $derived(data.partyId);

  let loading = $state(true);
  let error = $state<string | null>(null);
  let copiedCode = $state(false);
  let copiedLink = $state(false);
  let starting = $state(false);
  let startTimeout: ReturnType<typeof setTimeout> | null = null;

  let partyState = $derived($partyStore);
  let isHost = $derived(partyState.hostId === $user?.id);
  let memberCount = $derived(partyState.members.size);
  let canStart = $derived(isHost && memberCount >= 2);
  let membersArray = $derived(Array.from(partyState.members.values()));
  let isStarting = $derived(starting || partyState.status === 'starting');

  onMount(async () => {
    try {
      // Ensure user session exists
      if (!$user) {
        await authStore.createGuest();
      }

      // Join via socket so new invitees can enter directly from /party/{id}
      await partyStore.joinParty(partyId);
    } catch (e) {
      console.error('Failed to load party:', e);
      error = e instanceof Error ? e.message : 'Failed to join party';
    } finally {
      loading = false;
    }
  });

  onMount(() => {
    const unsubError = socketClient.on<{ code: string; message: string }>('party:error', () => {
      if (starting) {
        clearStartPending();
      }
    });

    return () => {
      unsubError();
      clearStartPending();
    };
  });

  $effect(() => {
    if ((partyState.status === 'starting' || partyState.status === 'in_game') && starting) {
      clearStartPending();
    }
  });

  $effect(() => {
    if (loading || error || partyState.partyId !== partyId) {
      return;
    }

    if (partyState.status === 'in_game' && partyState.currentGameId) {
      goto(`/game/${partyState.currentGameId}`);
    }
  });

  function clearStartPending() {
    starting = false;
    if (startTimeout) {
      clearTimeout(startTimeout);
      startTimeout = null;
    }
  }

  async function copyCode() {
    const code = partyState.joinCode;
    if (!code) return;

    try {
      await navigator.clipboard.writeText(code);
      copiedCode = true;
      toast.success('Code copied to clipboard!');
      setTimeout(() => {
        copiedCode = false;
      }, 2000);
    } catch {
      toast.error('Failed to copy code');
    }
  }

  async function copyLink() {
    const invitePath = `/party/${partyState.partyId || partyId}`;
    const inviteLink =
      typeof window === 'undefined' ? invitePath : new URL(invitePath, window.location.origin).toString();

    try {
      await navigator.clipboard.writeText(inviteLink);
      copiedLink = true;
      toast.success('Invite link copied to clipboard!');
      setTimeout(() => {
        copiedLink = false;
      }, 2000);
    } catch {
      toast.error('Failed to copy invite link');
    }
  }

  function handleStart() {
    if (isStarting) return;

    starting = true;

    if (startTimeout) {
      clearTimeout(startTimeout);
    }

    startTimeout = setTimeout(() => {
      if (starting || partyState.status === 'starting') {
        clearStartPending();
        toast.error('Failed to start game');
      }
    }, 15000);

    partyStore.startGame();
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
{:else if partyState.partyId === partyId}
  <div class="container mx-auto px-4 py-6 max-w-2xl space-y-6">
    <!-- Header -->
    <div class="text-center space-y-2">
      <h1 class="text-2xl font-bold">Party Lobby</h1>
      <p class="text-muted-foreground">
        Share the code or link below to invite friends
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
        </div>

        <div class="mt-4 flex flex-wrap items-center justify-center gap-2">
          <Button variant="outline" class="gap-2" onclick={copyCode}>
            {#if copiedCode}
              <CheckIcon class="h-4 w-4 text-green-500" />
            {:else}
              <CopyIcon class="h-4 w-4" />
            {/if}
            Copy Code
          </Button>

          <Button variant="outline" class="gap-2" onclick={copyLink}>
            {#if copiedLink}
              <CheckIcon class="h-4 w-4 text-green-500" />
            {:else}
              <LinkIcon class="h-4 w-4" />
            {/if}
            Copy Link
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
                  aria-label="Kick {member.display_name}"
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
          disabled={!canStart || isStarting}
          loading={isStarting}
          onclick={handleStart}
        >
          {#if !isStarting}
            <PlayIcon class="h-5 w-5" />
          {/if}
          {#if memberCount < 2}
            Need at least 2 players
          {:else if isStarting}
            Starting...
          {:else}
            Start Game
          {/if}
        </Button>
      {:else if partyState.status === 'starting'}
        <div class="flex flex-col items-center justify-center gap-3 py-3 text-center text-muted-foreground">
          <Spinner class="size-6 text-primary" />
          <p>Host is starting the game...</p>
        </div>
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
{:else}
  <div class="flex items-center justify-center h-64">
    <Spinner class="size-10 text-primary" />
  </div>
{/if}
