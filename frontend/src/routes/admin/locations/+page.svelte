<script lang="ts">
  import { onMount } from 'svelte';
  import { adminApi, type ReviewQueueResponse } from '$lib/api/admin';
  import { toast } from 'svelte-sonner';
  import * as Table from '$lib/components/ui/table';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import CheckIcon from '@lucide/svelte/icons/check';
  import XIcon from '@lucide/svelte/icons/x';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import SEO from '$lib/components/SEO.svelte';

  let data: ReviewQueueResponse | null = $state(null);
  let loading = $state(true);
  let page = $state(1);
  let statusFilter = $state<string | undefined>(undefined);
  let actionLoading = $state<string | null>(null);

  async function loadData() {
    loading = true;
    try {
      data = await adminApi.getReviewQueue({ page, status: statusFilter });
    } catch (e) {
      toast.error('Failed to load review queue');
      console.error('Failed to load review queue:', e);
    } finally {
      loading = false;
    }
  }

  async function quickAction(locationId: string, status: 'approved' | 'rejected') {
    actionLoading = locationId;
    try {
      await adminApi.updateReviewStatus(locationId, status);
      toast.success(`Location ${status === 'approved' ? 'approved' : 'rejected'}`);
      await loadData();
    } catch (e) {
      toast.error(`Failed to ${status === 'approved' ? 'approve' : 'reject'} location`);
      console.error('Failed to update status:', e);
    } finally {
      actionLoading = null;
    }
  }

  function getStatusBadgeVariant(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    switch (status) {
      case 'flagged': return 'destructive';
      case 'pending': return 'secondary';
      case 'approved': return 'default';
      default: return 'outline';
    }
  }

  function openGoogleMaps(lat: number, lng: number) {
    window.open(`https://www.google.com/maps?q=${lat},${lng}`, '_blank');
  }

  onMount(() => {
    loadData();
  });

  // Reload when page or filter changes
  $effect(() => {
    page;
    statusFilter;
    loadData();
  });
</script>

<SEO title="Admin Review Queue" noindex />

<div class="space-y-6">
  <!-- Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h1 class="text-3xl font-bold">Review Queue</h1>
      <p class="text-muted-foreground">
        Review and approve or reject flagged locations
      </p>
    </div>
    
    <div class="flex items-center gap-2">
      <select
        class="h-9 w-40 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus:outline-none focus:ring-2 focus:ring-ring"
        onchange={(e) => {
          const value = e.currentTarget.value;
          statusFilter = value === 'all' ? undefined : value;
          page = 1;
        }}
      >
        <option value="all">All Statuses</option>
        <option value="flagged">Flagged</option>
        <option value="pending">Pending</option>
      </select>
    </div>
  </div>

  <!-- Table -->
  <div class="border rounded-lg">
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head class="w-32">Preview</Table.Head>
          <Table.Head>Location</Table.Head>
          <Table.Head>Status</Table.Head>
          <Table.Head class="text-center">Reports</Table.Head>
          <Table.Head>Last Reason</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#if loading}
          {#each Array(5) as _}
            <Table.Row>
              <Table.Cell><Skeleton class="h-16 w-24 rounded" /></Table.Cell>
              <Table.Cell><Skeleton class="h-4 w-32" /></Table.Cell>
              <Table.Cell><Skeleton class="h-6 w-20 rounded-full" /></Table.Cell>
              <Table.Cell><Skeleton class="h-4 w-8 mx-auto" /></Table.Cell>
              <Table.Cell><Skeleton class="h-4 w-24" /></Table.Cell>
              <Table.Cell><Skeleton class="h-8 w-32 ml-auto" /></Table.Cell>
            </Table.Row>
          {/each}
        {:else if !data?.locations.length}
          <Table.Row>
            <Table.Cell colspan={6} class="h-32 text-center">
              <div class="flex flex-col items-center gap-2 text-muted-foreground">
                <MapPinIcon class="size-8" />
                <p>No locations in the review queue</p>
              </div>
            </Table.Cell>
          </Table.Row>
        {:else}
          {#each data.locations as location}
            <Table.Row>
              <!-- Thumbnail -->
              <Table.Cell>
                <a href="/admin/locations/{location.id}" class="block">
                  <img
                    src="https://maps.googleapis.com/maps/api/streetview?size=120x80&pano={location.panorama_id}&key={import.meta.env.VITE_GOOGLE_MAPS_API_KEY}"
                    alt="Street View preview"
                    class="rounded border object-cover w-24 h-16 hover:opacity-80 transition-opacity"
                  />
                </a>
              </Table.Cell>

              <!-- Location Info -->
              <Table.Cell>
                <div class="space-y-1">
                  <a 
                    href="/admin/locations/{location.id}" 
                    class="font-mono text-sm hover:underline"
                  >
                    {location.id}
                  </a>
                  <div class="text-xs text-muted-foreground">
                    {location.lat.toFixed(4)}, {location.lng.toFixed(4)}
                    {#if location.country_code}
                      <span class="ml-1">({location.country_code})</span>
                    {/if}
                  </div>
                </div>
              </Table.Cell>

              <!-- Status -->
              <Table.Cell>
                <Badge variant={getStatusBadgeVariant(location.review_status)}>
                  {location.review_status}
                </Badge>
              </Table.Cell>

              <!-- Report Count -->
              <Table.Cell class="text-center">
                <span class="font-medium {location.failure_count >= 3 ? 'text-red-500' : ''}">
                  {location.failure_count}
                </span>
              </Table.Cell>

              <!-- Last Reason -->
              <Table.Cell>
                {#if location.last_report_reason}
                  <span class="text-sm text-muted-foreground capitalize">
                    {location.last_report_reason.replace('_', ' ')}
                  </span>
                {:else}
                  <span class="text-sm text-muted-foreground">-</span>
                {/if}
              </Table.Cell>

              <!-- Actions -->
              <Table.Cell class="text-right">
                <div class="flex items-center justify-end gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => openGoogleMaps(location.lat, location.lng)}
                    title="Open in Google Maps"
                  >
                    <ExternalLinkIcon class="size-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    href="/admin/locations/{location.id}"
                    title="View Details"
                  >
                    <EyeIcon class="size-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="text-green-600 hover:text-green-700 hover:bg-green-50"
                    onclick={() => quickAction(location.id, 'approved')}
                    disabled={actionLoading === location.id}
                    title="Approve"
                  >
                    <CheckIcon class="size-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    class="text-red-600 hover:text-red-700 hover:bg-red-50"
                    onclick={() => quickAction(location.id, 'rejected')}
                    disabled={actionLoading === location.id}
                    title="Reject"
                  >
                    <XIcon class="size-4" />
                  </Button>
                </div>
              </Table.Cell>
            </Table.Row>
          {/each}
        {/if}
      </Table.Body>
    </Table.Root>
  </div>

  <!-- Pagination -->
  {#if data && data.total_pages > 1}
    <div class="flex items-center justify-between">
      <p class="text-sm text-muted-foreground">
        Showing {((page - 1) * data.per_page) + 1} - {Math.min(page * data.per_page, data.total)} of {data.total} locations
      </p>
      <div class="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onclick={() => page--}
          disabled={page <= 1}
        >
          <ChevronLeftIcon class="size-4" />
          Previous
        </Button>
        <span class="text-sm text-muted-foreground px-2">
          Page {page} of {data.total_pages}
        </span>
        <Button
          variant="outline"
          size="sm"
          onclick={() => page++}
          disabled={page >= data.total_pages}
        >
          Next
          <ChevronRightIcon class="size-4" />
        </Button>
      </div>
    </div>
  {/if}
</div>
