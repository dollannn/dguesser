<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { user, isGuest, authStore } from '$lib/stores/auth';
  import { authModalOpen } from '$lib/stores/authModal';
  import { gameStore } from '$lib/socket/game';
  import ConnectionStatus from './ConnectionStatus.svelte';
  import { Button } from '$lib/components/ui/button';
  import { Avatar, AvatarFallback, AvatarImage } from '$lib/components/ui/avatar';
  import { Badge } from '$lib/components/ui/badge';
  import { Separator } from '$lib/components/ui/separator';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
  import GlobeIcon from '@lucide/svelte/icons/globe';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import UserIcon from '@lucide/svelte/icons/user';
  import HistoryIcon from '@lucide/svelte/icons/history';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import PlayIcon from '@lucide/svelte/icons/play';
  import SettingsIcon from '@lucide/svelte/icons/settings';
  import MenuIcon from '@lucide/svelte/icons/menu';

  // Game state detection for pill header
  let gameState = $derived($gameStore);
  let isInGame = $derived(
    gameState.status === 'lobby' ||
      gameState.status === 'playing' ||
      gameState.status === 'round_end'
  );

  async function handleLogout() {
    await authStore.logout();
  }

  function isActive(path: string): boolean {
    return $page.url.pathname === path || $page.url.pathname.startsWith(path + '/');
  }
</script>

{#if isInGame}
  <!-- Floating pill header for in-game mode -->
  <header class="fixed top-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 h-12 px-4 rounded-full border border-border/40 bg-background/80 backdrop-blur-xl shadow-lg supports-[backdrop-filter]:bg-background/60">
    <!-- Logo -->
    <a href="/" class="flex items-center gap-2 group">
      <div class="flex items-center justify-center size-7 rounded-lg bg-primary text-primary-foreground shadow-sm group-hover:shadow transition-shadow">
        <GlobeIcon class="size-3.5" />
      </div>
      <span class="font-semibold text-sm tracking-tight hidden sm:block">DGuesser</span>
    </a>

    <div class="w-px h-6 bg-border/50"></div>

    <!-- User Section -->
    {#if $user}
      <div class="flex items-center gap-2">
        <ConnectionStatus minimal />
        
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <button 
                class="flex items-center gap-2 rounded-full p-0.5 pr-2 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                {...props}
              >
                <Avatar class="size-7 border border-border/50">
                  {#if $user.avatar_url}
                    <AvatarImage src={$user.avatar_url} alt={$user.display_name} />
                  {/if}
                  <AvatarFallback class="bg-muted text-muted-foreground text-xs font-medium">
                    {$user.display_name.charAt(0).toUpperCase()}
                  </AvatarFallback>
                </Avatar>
                <span class="hidden sm:block text-sm font-medium max-w-[160px] truncate">
                  {$user.display_name}
                </span>
                {#if $isGuest}
                  <Badge variant="secondary" class="text-[10px] px-1.5 py-0">Guest</Badge>
                {/if}
              </button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content class="w-56" align="end" sideOffset={8}>
            <DropdownMenu.Label>
              <div class="flex flex-col gap-1">
                <span class="font-medium">{$user.display_name}</span>
                {#if $user.email}
                  <span class="text-xs font-normal text-muted-foreground">{$user.email}</span>
                {:else if $isGuest}
                  <span class="text-xs font-normal text-muted-foreground">Playing as guest</span>
                {/if}
              </div>
            </DropdownMenu.Label>
            <DropdownMenu.Separator />

            {#if $isGuest}
              <DropdownMenu.Item onSelect={() => authModalOpen.open()}>
                <UserIcon />
                Sign in to save progress
              </DropdownMenu.Item>
            {:else}
              <DropdownMenu.Item onSelect={() => goto('/account')}>
                <SettingsIcon />
                Account Settings
              </DropdownMenu.Item>
              <DropdownMenu.Separator />
              <DropdownMenu.Item onSelect={handleLogout} variant="destructive">
                <LogOutIcon />
                Log out
              </DropdownMenu.Item>
            {/if}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </div>
    {:else}
      <Button onclick={() => authModalOpen.open()} size="sm">
        Sign In
      </Button>
    {/if}
  </header>
{:else}
  <!-- Normal full-width header -->
  <header class="sticky top-0 z-50 w-full border-b border-border/40 bg-background/80 backdrop-blur-xl supports-[backdrop-filter]:bg-background/60">
    <div class="container mx-auto px-4 flex h-14 items-center">
      <!-- Logo -->
      <a href="/" class="flex items-center gap-2.5 mr-6 group">
        <div class="flex items-center justify-center size-8 rounded-lg bg-primary text-primary-foreground shadow-sm group-hover:shadow transition-shadow">
          <GlobeIcon class="size-4" />
        </div>
        <span class="font-semibold text-lg tracking-tight">DGuesser</span>
      </a>

      <!-- Desktop Navigation -->
      {#if $user}
        <nav class="hidden md:flex items-center gap-1">
          <a 
            href="/play" 
            class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors
              {isActive('/play') 
                ? 'bg-accent text-accent-foreground' 
                : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'}"
          >
            <PlayIcon class="size-4" />
            Play
          </a>
          <a 
            href="/leaderboard" 
            class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors
              {isActive('/leaderboard') 
                ? 'bg-accent text-accent-foreground' 
                : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'}"
          >
            <TrophyIcon class="size-4" />
            Leaderboard
          </a>
          <a 
            href="/history" 
            class="inline-flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-colors
              {isActive('/history') 
                ? 'bg-accent text-accent-foreground' 
                : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'}"
          >
            <HistoryIcon class="size-4" />
            History
          </a>
        </nav>
      {/if}

      <!-- Right Section -->
      <div class="ml-auto flex items-center gap-2">
        {#if $user}
          <ConnectionStatus />
          
          <DropdownMenu.Root>
            <DropdownMenu.Trigger>
              {#snippet child({ props })}
                <button 
                  class="flex items-center gap-2 rounded-full p-1 pr-2 transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  {...props}
                >
                  <Avatar class="size-8 border border-border/50">
                    {#if $user.avatar_url}
                      <AvatarImage src={$user.avatar_url} alt={$user.display_name} />
                    {/if}
                    <AvatarFallback class="bg-muted text-muted-foreground text-sm font-medium">
                      {$user.display_name.charAt(0).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <span class="hidden sm:block text-sm font-medium max-w-[100px] truncate">
                    {$user.display_name}
                  </span>
                  {#if $isGuest}
                    <Badge variant="secondary" class="text-[10px] px-1.5 py-0">Guest</Badge>
                  {/if}
                </button>
              {/snippet}
            </DropdownMenu.Trigger>
            <DropdownMenu.Content class="w-56" align="end" sideOffset={8}>
              <DropdownMenu.Label>
                <div class="flex flex-col gap-1">
                  <span class="font-medium">{$user.display_name}</span>
                  {#if $user.email}
                    <span class="text-xs font-normal text-muted-foreground">{$user.email}</span>
                  {:else if $isGuest}
                    <span class="text-xs font-normal text-muted-foreground">Playing as guest</span>
                  {/if}
                </div>
              </DropdownMenu.Label>
              <DropdownMenu.Separator />
              
              <!-- Mobile-only nav items -->
              <DropdownMenu.Group class="md:hidden">
                <DropdownMenu.Item onSelect={() => goto('/play')}>
                  <PlayIcon />
                  Play
                </DropdownMenu.Item>
                <DropdownMenu.Item onSelect={() => goto('/leaderboard')}>
                  <TrophyIcon />
                  Leaderboard
                </DropdownMenu.Item>
                <DropdownMenu.Item onSelect={() => goto('/history')}>
                  <HistoryIcon />
                  History
                </DropdownMenu.Item>
                <DropdownMenu.Separator />
              </DropdownMenu.Group>

              {#if $isGuest}
                <DropdownMenu.Item onSelect={() => authModalOpen.open()}>
                  <UserIcon />
                  Sign in to save progress
                </DropdownMenu.Item>
              {:else}
                <DropdownMenu.Item onSelect={() => goto('/account')}>
                  <SettingsIcon />
                  Account Settings
                </DropdownMenu.Item>
                <DropdownMenu.Separator />
                <DropdownMenu.Item onSelect={handleLogout} variant="destructive">
                  <LogOutIcon />
                  Log out
                </DropdownMenu.Item>
              {/if}
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        {:else}
          <Button onclick={() => authModalOpen.open()} size="sm">
            Sign In
          </Button>
        {/if}
      </div>
    </div>
  </header>
{/if}
