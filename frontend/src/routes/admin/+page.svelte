<script lang="ts">
  import { onMount } from 'svelte';
  import { adminApi, type AdminStats } from '$lib/api/admin';
  import { toast } from 'svelte-sonner';
  import * as Card from '$lib/components/ui/card';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { Button } from '$lib/components/ui/button';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import FlagIcon from '@lucide/svelte/icons/flag';
  import AlertTriangleIcon from '@lucide/svelte/icons/alert-triangle';
  import CheckCircleIcon from '@lucide/svelte/icons/check-circle';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import DatabaseIcon from '@lucide/svelte/icons/database';

  let stats: AdminStats | null = $state(null);
  let loading = $state(true);
  let refreshing = $state(false);

  async function loadStats() {
    try {
      stats = await adminApi.getStats();
    } catch (e) {
      toast.error('Failed to load statistics');
      console.error('Failed to load stats:', e);
    } finally {
      loading = false;
      refreshing = false;
    }
  }

  async function refresh() {
    refreshing = true;
    await loadStats();
    toast.success('Statistics refreshed');
  }

  onMount(() => {
    loadStats();
  });
</script>

<svelte:head>
  <title>Admin Dashboard - DGuesser</title>
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold">Dashboard</h1>
      <p class="text-muted-foreground">Overview of location statistics and pending reviews</p>
    </div>
    <Button onclick={refresh} variant="outline" disabled={refreshing}>
      <RefreshCwIcon class="size-4 mr-2 {refreshing ? 'animate-spin' : ''}" />
      Refresh
    </Button>
  </div>

  <!-- Stats Cards -->
  <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
    <!-- Total Locations -->
    <Card.Root>
      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
        <Card.Title class="text-sm font-medium">Total Locations</Card.Title>
        <DatabaseIcon class="size-4 text-muted-foreground" />
      </Card.Header>
      <Card.Content>
        {#if loading}
          <Skeleton class="h-8 w-24" />
        {:else}
          <div class="text-2xl font-bold">{stats?.total_locations.toLocaleString()}</div>
          <p class="text-xs text-muted-foreground">
            {stats?.active_locations.toLocaleString()} active
          </p>
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- Pending Review -->
    <Card.Root>
      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
        <Card.Title class="text-sm font-medium">Pending Review</Card.Title>
        <AlertTriangleIcon class="size-4 text-amber-500" />
      </Card.Header>
      <Card.Content>
        {#if loading}
          <Skeleton class="h-8 w-16" />
        {:else}
          <div class="text-2xl font-bold">{stats?.pending_review.toLocaleString()}</div>
          <p class="text-xs text-muted-foreground">
            Locations needing review
          </p>
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- Recent Reports -->
    <Card.Root>
      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
        <Card.Title class="text-sm font-medium">Recent Reports</Card.Title>
        <FlagIcon class="size-4 text-red-500" />
      </Card.Header>
      <Card.Content>
        {#if loading}
          <Skeleton class="h-8 w-16" />
        {:else}
          <div class="text-2xl font-bold">{stats?.recent_reports.toLocaleString()}</div>
          <p class="text-xs text-muted-foreground">
            In the last 7 days
          </p>
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- Approved Rate -->
    <Card.Root>
      <Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
        <Card.Title class="text-sm font-medium">Approved</Card.Title>
        <CheckCircleIcon class="size-4 text-green-500" />
      </Card.Header>
      <Card.Content>
        {#if loading}
          <Skeleton class="h-8 w-16" />
        {:else}
          {@const approved = stats?.by_review_status.approved ?? 0}
          {@const total = stats?.total_locations ?? 1}
          <div class="text-2xl font-bold">{((approved / total) * 100).toFixed(1)}%</div>
          <p class="text-xs text-muted-foreground">
            {approved.toLocaleString()} locations
          </p>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>

  <!-- Quick Actions -->
  <div class="grid gap-4 md:grid-cols-2">
    <Card.Root>
      <Card.Header>
        <Card.Title>Review Queue</Card.Title>
        <Card.Description>
          Review flagged and pending locations
        </Card.Description>
      </Card.Header>
      <Card.Content>
        <div class="flex items-center gap-4">
          <div class="flex-1">
            {#if loading}
              <Skeleton class="h-4 w-32" />
            {:else}
              <p class="text-sm text-muted-foreground">
                {stats?.pending_review} locations waiting for review
              </p>
            {/if}
          </div>
          <Button href="/admin/locations">
            <MapPinIcon class="size-4 mr-2" />
            View Queue
          </Button>
        </div>
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Header>
        <Card.Title>All Reports</Card.Title>
        <Card.Description>
          View all user-submitted reports
        </Card.Description>
      </Card.Header>
      <Card.Content>
        <div class="flex items-center gap-4">
          <div class="flex-1">
            {#if loading}
              <Skeleton class="h-4 w-32" />
            {:else}
              <p class="text-sm text-muted-foreground">
                {stats?.recent_reports} reports in the last week
              </p>
            {/if}
          </div>
          <Button href="/admin/reports" variant="outline">
            <FlagIcon class="size-4 mr-2" />
            View Reports
          </Button>
        </div>
      </Card.Content>
    </Card.Root>
  </div>

  <!-- Breakdown Cards -->
  <div class="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
    <!-- By Review Status -->
    <Card.Root>
      <Card.Header>
        <Card.Title class="text-base">By Review Status</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if loading}
          <div class="space-y-2">
            <Skeleton class="h-4 w-full" />
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="h-4 w-1/2" />
          </div>
        {:else}
          <div class="space-y-2">
            {#each Object.entries(stats?.by_review_status ?? {}) as [status, count]}
              <div class="flex items-center justify-between text-sm">
                <span class="capitalize">{status}</span>
                <span class="font-medium">{count.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- By Source -->
    <Card.Root>
      <Card.Header>
        <Card.Title class="text-base">By Source</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if loading}
          <div class="space-y-2">
            <Skeleton class="h-4 w-full" />
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="h-4 w-1/2" />
          </div>
        {:else}
          <div class="space-y-2">
            {#each Object.entries(stats?.by_source ?? {}) as [source, count]}
              <div class="flex items-center justify-between text-sm">
                <span class="capitalize">{source}</span>
                <span class="font-medium">{count.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        {/if}
      </Card.Content>
    </Card.Root>

    <!-- By Validation Status -->
    <Card.Root>
      <Card.Header>
        <Card.Title class="text-base">By Validation Status</Card.Title>
      </Card.Header>
      <Card.Content>
        {#if loading}
          <div class="space-y-2">
            <Skeleton class="h-4 w-full" />
            <Skeleton class="h-4 w-3/4" />
            <Skeleton class="h-4 w-1/2" />
          </div>
        {:else}
          <div class="space-y-2">
            {#each Object.entries(stats?.by_status ?? {}) as [status, count]}
              <div class="flex items-center justify-between text-sm">
                <span class="capitalize">{status.replace('_', ' ')}</span>
                <span class="font-medium">{count.toLocaleString()}</span>
              </div>
            {/each}
          </div>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>
</div>
