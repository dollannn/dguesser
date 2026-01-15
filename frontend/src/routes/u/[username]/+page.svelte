<script lang="ts">
  import { page } from '$app/stores';
  import { user as currentUser } from '$lib/stores/auth';
  import { usersApi, type UserProfile } from '$lib/api';
  import { formatScore } from '$lib/utils';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Avatar, AvatarFallback, AvatarImage } from '$lib/components/ui/avatar';
  import * as Card from '$lib/components/ui/card';
  import UserIcon from '@lucide/svelte/icons/user';
  import AtSignIcon from '@lucide/svelte/icons/at-sign';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import GamepadIcon from '@lucide/svelte/icons/gamepad-2';
  import StarIcon from '@lucide/svelte/icons/star';
  import CalendarIcon from '@lucide/svelte/icons/calendar';
  import SettingsIcon from '@lucide/svelte/icons/settings';

  let profile = $state<UserProfile | null>(null);
  let loading = $state(true);
  let error = $state('');

  // Load profile when username changes
  $effect(() => {
    const username = $page.params.username;
    if (username) {
      loadProfile(username);
    }
  });

  async function loadProfile(username: string) {
    loading = true;
    error = '';
    try {
      profile = await usersApi.getUserByUsername(username);
    } catch (e: any) {
      if (e.code === 'NOT_FOUND') {
        error = 'User not found';
      } else {
        error = e.message || 'Failed to load profile';
      }
    } finally {
      loading = false;
    }
  }

  let isOwnProfile = $derived(profile && $currentUser && profile.id === $currentUser.id);
</script>

<svelte:head>
  {#if profile}
    <title>{profile.display_name} (@{profile.username}) - DGuesser</title>
  {:else}
    <title>Profile - DGuesser</title>
  {/if}
</svelte:head>

<div class="max-w-2xl mx-auto px-4 py-8">
  {#if loading}
    <!-- Loading skeleton -->
    <div class="animate-pulse">
      <div class="flex items-center gap-6 mb-8">
        <div class="w-24 h-24 rounded-full bg-muted"></div>
        <div class="space-y-3 flex-1">
          <div class="h-8 bg-muted rounded w-48"></div>
          <div class="h-5 bg-muted rounded w-32"></div>
        </div>
      </div>
      <div class="grid grid-cols-3 gap-4">
        {#each [1, 2, 3] as _}
          <div class="h-24 bg-muted rounded-lg"></div>
        {/each}
      </div>
    </div>
  {:else if error}
    <!-- Error state -->
    <div class="text-center py-16">
      <UserIcon class="w-16 h-16 mx-auto text-muted-foreground mb-4" />
      <h1 class="text-2xl font-bold text-foreground mb-2">{error}</h1>
      <p class="text-muted-foreground mb-6">
        {#if error === 'User not found'}
          The user @{$page.params.username} doesn't exist or has been deleted.
        {:else}
          There was a problem loading this profile.
        {/if}
      </p>
      <Button variant="outline" href="/">Go Home</Button>
    </div>
  {:else if profile}
    <!-- Profile content -->
    <div class="space-y-6">
      <!-- Header -->
      <div class="flex items-start gap-6">
        <Avatar class="w-24 h-24 border-4 border-background shadow-lg">
          {#if profile.avatar_url}
            <AvatarImage src={profile.avatar_url} alt={profile.display_name} />
          {/if}
          <AvatarFallback class="text-3xl font-bold bg-muted">
            {profile.display_name.charAt(0).toUpperCase()}
          </AvatarFallback>
        </Avatar>

        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-3 flex-wrap">
            <h1 class="text-3xl font-bold truncate">{profile.display_name}</h1>
            {#if profile.is_guest}
              <Badge variant="secondary">Guest</Badge>
            {/if}
          </div>
          
          {#if profile.username}
            <p class="text-lg text-muted-foreground flex items-center gap-1 mt-1">
              <AtSignIcon class="w-4 h-4" />
              {profile.username}
            </p>
          {/if}

          {#if isOwnProfile}
            <div class="mt-4">
              <Button variant="outline" size="sm" href="/account">
                <SettingsIcon class="w-4 h-4 mr-2" />
                Edit Profile
              </Button>
            </div>
          {/if}
        </div>
      </div>

      <!-- Stats -->
      <Card.Root>
        <Card.Header>
          <Card.Title class="flex items-center gap-2">
            <TrophyIcon class="w-5 h-5 text-amber-500" />
            Statistics
          </Card.Title>
        </Card.Header>
        <Card.Content>
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <GamepadIcon class="w-8 h-8 mx-auto mb-2 text-muted-foreground" />
              <p class="text-3xl font-bold">{profile.games_played}</p>
              <p class="text-sm text-muted-foreground">Games Played</p>
            </div>
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <TrophyIcon class="w-8 h-8 mx-auto mb-2 text-amber-500" />
              <p class="text-3xl font-bold">{formatScore(profile.total_score)}</p>
              <p class="text-sm text-muted-foreground">Total Score</p>
            </div>
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <StarIcon class="w-8 h-8 mx-auto mb-2 text-yellow-500" />
              <p class="text-3xl font-bold">{formatScore(profile.best_score)}</p>
              <p class="text-sm text-muted-foreground">Best Score</p>
            </div>
          </div>

          {#if profile.games_played > 0}
            <div class="mt-4 pt-4 border-t border-border">
              <p class="text-sm text-muted-foreground text-center">
                Average score per game: <span class="font-medium text-foreground">
                  {formatScore(Math.round(profile.total_score / profile.games_played))}
                </span>
              </p>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>

      <!-- Leaderboard link -->
      <div class="text-center">
        <a href="/leaderboard" class="text-sm text-primary hover:underline">
          View Global Leaderboard
        </a>
      </div>
    </div>
  {/if}
</div>
