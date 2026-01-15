<script lang="ts">
  import { goto } from '$app/navigation';
  import { user, isGuest, authStore } from '$lib/stores/auth';
  import { authModalOpen } from '$lib/stores/authModal';
  import { usersApi, sessionsApi, type SessionInfo } from '$lib/api';
  import { toast } from 'svelte-sonner';
  import { formatScore } from '$lib/utils';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Label } from '$lib/components/ui/label';
  import { Badge } from '$lib/components/ui/badge';
  import { Separator } from '$lib/components/ui/separator';
  import { Avatar, AvatarFallback, AvatarImage } from '$lib/components/ui/avatar';
  import * as Card from '$lib/components/ui/card';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import UserIcon from '@lucide/svelte/icons/user';
  import AtSignIcon from '@lucide/svelte/icons/at-sign';
  import TrophyIcon from '@lucide/svelte/icons/trophy';
  import GamepadIcon from '@lucide/svelte/icons/gamepad-2';
  import StarIcon from '@lucide/svelte/icons/star';
  import MonitorSmartphoneIcon from '@lucide/svelte/icons/monitor-smartphone';
  import TrashIcon from '@lucide/svelte/icons/trash-2';
  import LogOutIcon from '@lucide/svelte/icons/log-out';
  import CheckIcon from '@lucide/svelte/icons/check';
  import XIcon from '@lucide/svelte/icons/x';
  import LoaderIcon from '@lucide/svelte/icons/loader';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';

  // Profile editing state
  let username = $state($user?.username ?? '');
  let displayName = $state($user?.display_name ?? '');
  let isEditingProfile = $state(false);
  let isSavingProfile = $state(false);
  let usernameError = $state('');

  // Sessions state
  let sessions = $state<SessionInfo[]>([]);
  let loadingSessions = $state(true);
  let revokingSession = $state<string | null>(null);

  // Delete account state
  let showDeleteDialog = $state(false);
  let isDeleting = $state(false);

  // Sync state with user changes
  $effect(() => {
    if ($user) {
      username = $user.username ?? '';
      displayName = $user.display_name;
    }
  });

  // Load sessions on mount
  $effect(() => {
    if ($user && !$isGuest) {
      loadSessions();
    }
  });

  async function loadSessions() {
    loadingSessions = true;
    try {
      const response = await sessionsApi.listSessions();
      sessions = response.sessions;
    } catch (e) {
      console.error('Failed to load sessions:', e);
    } finally {
      loadingSessions = false;
    }
  }

  function validateUsername(value: string): string | null {
    if (!value) return null; // Empty is ok (will keep current or none)
    if (value.length < 3) return 'Username must be at least 3 characters';
    if (value.length > 30) return 'Username must be at most 30 characters';
    if (value !== value.toLowerCase()) return 'Username must be lowercase';
    if (!/^[a-z0-9][a-z0-9_]*[a-z0-9]$|^[a-z0-9]{1,2}$/.test(value)) {
      return 'Username can only contain lowercase letters, numbers, and underscores';
    }
    if (value.includes('__')) return 'Username cannot contain consecutive underscores';
    return null;
  }

  function handleUsernameInput(e: Event) {
    const input = e.target as HTMLInputElement;
    username = input.value.toLowerCase().replace(/[^a-z0-9_]/g, '');
    usernameError = validateUsername(username) ?? '';
  }

  async function saveProfile() {
    if (usernameError) return;
    
    isSavingProfile = true;
    try {
      const updates: { username?: string; display_name?: string } = {};
      
      if (username !== ($user?.username ?? '')) {
        updates.username = username || undefined;
      }
      if (displayName !== $user?.display_name && displayName.length >= 3) {
        updates.display_name = displayName;
      }

      if (Object.keys(updates).length > 0) {
        const updated = await usersApi.updateProfile(updates);
        authStore.setUser({ ...$user!, ...updated });
        toast.success('Profile updated successfully');
      }
      isEditingProfile = false;
    } catch (e: any) {
      if (e.code === 'USERNAME_TAKEN') {
        usernameError = 'This username is already taken';
      } else if (e.code === 'RESERVED_USERNAME') {
        usernameError = 'This username is reserved';
      } else {
        toast.error(e.message || 'Failed to update profile');
      }
    } finally {
      isSavingProfile = false;
    }
  }

  function cancelEdit() {
    username = $user?.username ?? '';
    displayName = $user?.display_name ?? '';
    usernameError = '';
    isEditingProfile = false;
  }

  async function revokeSession(sessionId: string) {
    revokingSession = sessionId;
    try {
      await sessionsApi.revokeSession(sessionId);
      sessions = sessions.filter(s => s.id !== sessionId);
      toast.success('Session revoked');
    } catch (e: any) {
      toast.error(e.message || 'Failed to revoke session');
    } finally {
      revokingSession = null;
    }
  }

  async function revokeOtherSessions() {
    try {
      const result = await sessionsApi.revokeOtherSessions();
      sessions = sessions.filter(s => s.is_current);
      toast.success(result.message);
    } catch (e: any) {
      toast.error(e.message || 'Failed to revoke sessions');
    }
  }

  async function deleteAccount() {
    isDeleting = true;
    try {
      await usersApi.deleteAccount();
      await authStore.logout();
      toast.success('Your account has been scheduled for deletion');
      goto('/');
    } catch (e: any) {
      toast.error(e.message || 'Failed to delete account');
    } finally {
      isDeleting = false;
      showDeleteDialog = false;
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function parseUserAgent(ua: string | null): { device: string; browser: string } {
    if (!ua) return { device: 'Unknown device', browser: 'Unknown browser' };
    
    // Simple parsing - could be more sophisticated
    let device = 'Desktop';
    if (ua.includes('Mobile') || ua.includes('Android')) device = 'Mobile';
    else if (ua.includes('Tablet') || ua.includes('iPad')) device = 'Tablet';

    let browser = 'Unknown';
    if (ua.includes('Firefox')) browser = 'Firefox';
    else if (ua.includes('Edg')) browser = 'Edge';
    else if (ua.includes('Chrome')) browser = 'Chrome';
    else if (ua.includes('Safari')) browser = 'Safari';

    return { device, browser };
  }
</script>

<svelte:head>
  <title>Account Settings - DGuesser</title>
</svelte:head>

{#if !$user}
  <div class="max-w-2xl mx-auto px-4 py-16 text-center">
    <UserIcon class="w-16 h-16 mx-auto text-gray-300 mb-4" />
    <h1 class="text-2xl font-bold text-gray-900 mb-2">Sign in to access your account</h1>
    <p class="text-gray-600 mb-6">Create an account to track your progress and customize your profile.</p>
    <Button onclick={() => authModalOpen.open()}>Sign In</Button>
  </div>
{:else}
  <div class="max-w-4xl mx-auto px-4 py-8">
    <!-- Header -->
    <div class="mb-8">
      <h1 class="text-4xl font-bold text-gray-900 mb-2">Account Settings</h1>
      <p class="text-gray-600">Manage your profile, sessions, and account</p>
    </div>

    <div class="space-y-6">
      <!-- Profile Section -->
      <Card.Root>
        <Card.Header>
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-lg bg-primary/10">
                <UserIcon class="w-5 h-5 text-primary" />
              </div>
              <div>
                <Card.Title>Profile</Card.Title>
                <Card.Description>Your public profile information</Card.Description>
              </div>
            </div>
            {#if !isEditingProfile}
              <Button variant="outline" size="sm" onclick={() => isEditingProfile = true}>
                Edit
              </Button>
            {/if}
          </div>
        </Card.Header>
        <Card.Content>
          <div class="flex items-start gap-6">
            <!-- Avatar -->
            <Avatar class="w-20 h-20 border-2 border-border">
              {#if $user.avatar_url}
                <AvatarImage src={$user.avatar_url} alt={$user.display_name} />
              {/if}
              <AvatarFallback class="text-2xl font-medium bg-muted">
                {$user.display_name.charAt(0).toUpperCase()}
              </AvatarFallback>
            </Avatar>

            <!-- Profile fields -->
            <div class="flex-1 space-y-4">
              {#if isEditingProfile}
                <!-- Edit mode -->
                <div class="space-y-4">
                  <div class="space-y-2">
                    <Label for="username">Username</Label>
                    <div class="relative">
                      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground">@</span>
                      <Input
                        id="username"
                        value={username}
                        oninput={handleUsernameInput}
                        placeholder="coolplayer42"
                        class="pl-8 {usernameError ? 'border-destructive' : ''}"
                      />
                    </div>
                    {#if usernameError}
                      <p class="text-sm text-destructive">{usernameError}</p>
                    {:else}
                      <p class="text-sm text-muted-foreground">
                        Your profile URL: dguesser.com/u/{username || 'username'}
                      </p>
                    {/if}
                  </div>

                  <div class="space-y-2">
                    <Label for="displayName">Display Name</Label>
                    <Input
                      id="displayName"
                      bind:value={displayName}
                      placeholder="Cool Player"
                      minlength={3}
                      maxlength={50}
                    />
                    <p class="text-sm text-muted-foreground">
                      This is how your name appears to other players
                    </p>
                  </div>

                  <div class="flex gap-2">
                    <Button 
                      onclick={saveProfile} 
                      disabled={isSavingProfile || !!usernameError}
                      size="sm"
                    >
                      {#if isSavingProfile}
                        <LoaderIcon class="w-4 h-4 mr-2 animate-spin" />
                      {:else}
                        <CheckIcon class="w-4 h-4 mr-2" />
                      {/if}
                      Save Changes
                    </Button>
                    <Button variant="ghost" size="sm" onclick={cancelEdit}>
                      Cancel
                    </Button>
                  </div>
                </div>
              {:else}
                <!-- View mode -->
                <div class="space-y-3">
                  <div>
                    <p class="text-sm text-muted-foreground">Username</p>
                    {#if $user.username}
                      <p class="font-medium flex items-center gap-2">
                        <AtSignIcon class="w-4 h-4 text-muted-foreground" />
                        {$user.username}
                        <a 
                          href="/u/{$user.username}" 
                          class="text-xs text-primary hover:underline inline-flex items-center gap-1"
                        >
                          View profile <ExternalLinkIcon class="w-3 h-3" />
                        </a>
                      </p>
                    {:else}
                      <p class="text-muted-foreground italic">No username set</p>
                    {/if}
                  </div>
                  <div>
                    <p class="text-sm text-muted-foreground">Display Name</p>
                    <p class="font-medium">{$user.display_name}</p>
                  </div>
                  {#if $user.email}
                    <div>
                      <p class="text-sm text-muted-foreground">Email</p>
                      <p class="font-medium">{$user.email}</p>
                    </div>
                  {/if}
                  {#if $isGuest}
                    <div class="pt-2">
                      <Badge variant="secondary">Guest Account</Badge>
                      <p class="text-sm text-muted-foreground mt-1">
                        Sign in with Google or Microsoft to save your progress permanently
                      </p>
                      <Button size="sm" variant="outline" class="mt-2" onclick={() => authModalOpen.open()}>
                        Sign in to save progress
                      </Button>
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        </Card.Content>
      </Card.Root>

      <!-- Statistics Section -->
      <Card.Root>
        <Card.Header>
          <div class="flex items-center gap-3">
            <div class="p-2 rounded-lg bg-amber-500/10">
              <TrophyIcon class="w-5 h-5 text-amber-500" />
            </div>
            <div>
              <Card.Title>Statistics</Card.Title>
              <Card.Description>Your gameplay statistics</Card.Description>
            </div>
          </div>
        </Card.Header>
        <Card.Content>
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <GamepadIcon class="w-8 h-8 mx-auto mb-2 text-muted-foreground" />
              <p class="text-3xl font-bold">{$user.games_played}</p>
              <p class="text-sm text-muted-foreground">Games Played</p>
            </div>
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <TrophyIcon class="w-8 h-8 mx-auto mb-2 text-amber-500" />
              <p class="text-3xl font-bold">{formatScore($user.total_score)}</p>
              <p class="text-sm text-muted-foreground">Total Score</p>
            </div>
            <div class="p-4 rounded-lg bg-muted/50 text-center">
              <StarIcon class="w-8 h-8 mx-auto mb-2 text-yellow-500" />
              <p class="text-3xl font-bold">{formatScore($user.best_score)}</p>
              <p class="text-sm text-muted-foreground">Best Score</p>
            </div>
          </div>
          <div class="mt-4 text-center">
            <a href="/leaderboard" class="text-sm text-primary hover:underline">
              View Leaderboard
            </a>
          </div>
        </Card.Content>
      </Card.Root>

      <!-- Sessions Section (only for authenticated users) -->
      {#if !$isGuest}
        <Card.Root>
          <Card.Header>
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="p-2 rounded-lg bg-blue-500/10">
                  <MonitorSmartphoneIcon class="w-5 h-5 text-blue-500" />
                </div>
                <div>
                  <Card.Title>Active Sessions</Card.Title>
                  <Card.Description>Devices where you're logged in</Card.Description>
                </div>
              </div>
              {#if sessions.length > 1}
                <Button variant="outline" size="sm" onclick={revokeOtherSessions}>
                  <LogOutIcon class="w-4 h-4 mr-2" />
                  Sign out other devices
                </Button>
              {/if}
            </div>
          </Card.Header>
          <Card.Content>
            {#if loadingSessions}
              <div class="space-y-3">
                {#each [1, 2] as _}
                  <div class="animate-pulse flex items-center gap-4 p-3 rounded-lg bg-muted/30">
                    <div class="w-10 h-10 rounded-full bg-muted"></div>
                    <div class="flex-1 space-y-2">
                      <div class="h-4 bg-muted rounded w-1/3"></div>
                      <div class="h-3 bg-muted rounded w-1/2"></div>
                    </div>
                  </div>
                {/each}
              </div>
            {:else if sessions.length === 0}
              <p class="text-muted-foreground text-center py-4">No active sessions</p>
            {:else}
              <div class="space-y-3">
                {#each sessions as session (session.id)}
                  {@const ua = parseUserAgent(session.user_agent)}
                  <div class="flex items-center gap-4 p-3 rounded-lg bg-muted/30 {session.is_current ? 'ring-2 ring-primary/20' : ''}">
                    <div class="p-2 rounded-full bg-background">
                      <MonitorSmartphoneIcon class="w-5 h-5 text-muted-foreground" />
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <p class="font-medium truncate">{ua.device} - {ua.browser}</p>
                        {#if session.is_current}
                          <Badge variant="secondary" class="text-xs">Current</Badge>
                        {/if}
                      </div>
                      <p class="text-sm text-muted-foreground truncate">
                        {session.ip_address ?? 'Unknown IP'} · Last active {formatDate(session.last_accessed_at)}
                      </p>
                    </div>
                    {#if !session.is_current}
                      <Button 
                        variant="ghost" 
                        size="sm"
                        onclick={() => revokeSession(session.id)}
                        disabled={revokingSession === session.id}
                      >
                        {#if revokingSession === session.id}
                          <LoaderIcon class="w-4 h-4 animate-spin" />
                        {:else}
                          <XIcon class="w-4 h-4" />
                        {/if}
                      </Button>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </Card.Content>
        </Card.Root>
      {/if}

      <!-- Danger Zone -->
      {#if !$isGuest}
        <Card.Root class="border-destructive/50">
          <Card.Header>
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-lg bg-destructive/10">
                <TrashIcon class="w-5 h-5 text-destructive" />
              </div>
              <div>
                <Card.Title class="text-destructive">Danger Zone</Card.Title>
                <Card.Description>Irreversible actions</Card.Description>
              </div>
            </div>
          </Card.Header>
          <Card.Content>
            <div class="flex items-center justify-between p-4 rounded-lg bg-destructive/5 border border-destructive/20">
              <div>
                <p class="font-medium">Delete Account</p>
                <p class="text-sm text-muted-foreground">
                  Your account will be scheduled for deletion. You have 30 days to recover it.
                </p>
              </div>
              <AlertDialog.Root bind:open={showDeleteDialog}>
                <AlertDialog.Trigger>
                  {#snippet child({ props })}
                    <Button variant="destructive" size="sm" {...props}>
                      Delete Account
                    </Button>
                  {/snippet}
                </AlertDialog.Trigger>
                <AlertDialog.Portal>
                  <AlertDialog.Overlay />
                  <AlertDialog.Content>
                    <AlertDialog.Header>
                      <AlertDialog.Title>Are you absolutely sure?</AlertDialog.Title>
                      <AlertDialog.Description>
                        This will schedule your account for deletion. Your username will be released,
                        and your data will be permanently deleted after 30 days. You can recover your
                        account by signing in again within 30 days.
                      </AlertDialog.Description>
                    </AlertDialog.Header>
                    <AlertDialog.Footer>
                      <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                      <AlertDialog.Action onclick={deleteAccount} disabled={isDeleting}>
                        {#if isDeleting}
                          <LoaderIcon class="w-4 h-4 mr-2 animate-spin" />
                        {/if}
                        Yes, delete my account
                      </AlertDialog.Action>
                    </AlertDialog.Footer>
                  </AlertDialog.Content>
                </AlertDialog.Portal>
              </AlertDialog.Root>
            </div>
          </Card.Content>
        </Card.Root>
      {/if}
    </div>
  </div>
{/if}
