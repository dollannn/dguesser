<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { adminApi, type LocationDetail, type ReviewStatus } from '$lib/api/admin';
  import { toast } from 'svelte-sonner';
  import * as Card from '$lib/components/ui/card';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import { Separator } from '$lib/components/ui/separator';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import CheckIcon from '@lucide/svelte/icons/check';
  import XIcon from '@lucide/svelte/icons/x';
  import FlagIcon from '@lucide/svelte/icons/flag';
  import MapIcon from '@lucide/svelte/icons/map';
  import CalendarIcon from '@lucide/svelte/icons/calendar';
  import AlertTriangleIcon from '@lucide/svelte/icons/alert-triangle';

  let location: LocationDetail | null = $state(null);
  let loading = $state(true);
  let actionLoading = $state(false);
  let showRejectDialog = $state(false);

  // Get location ID from page params
  const locationId = $derived($page.params.id ?? '');

  async function loadLocation() {
    if (!locationId) return;
    try {
      location = await adminApi.getLocationDetail(locationId);
    } catch (e) {
      toast.error('Failed to load location details');
      console.error('Failed to load location:', e);
    } finally {
      loading = false;
    }
  }

  async function updateStatus(status: ReviewStatus) {
    actionLoading = true;
    try {
      await adminApi.updateReviewStatus(locationId, status);
      toast.success(`Location ${status === 'approved' ? 'approved' : status === 'rejected' ? 'rejected' : 'flagged'}`);
      await loadLocation();
    } catch (e) {
      toast.error('Failed to update status');
      console.error('Failed to update status:', e);
    } finally {
      actionLoading = false;
      showRejectDialog = false;
    }
  }

  function openGoogleMaps() {
    if (location) {
      window.open(`https://www.google.com/maps?q=${location.lat},${location.lng}`, '_blank');
    }
  }

  function getStatusBadgeVariant(status: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    switch (status) {
      case 'flagged': return 'destructive';
      case 'pending': return 'secondary';
      case 'approved': return 'default';
      case 'rejected': return 'outline';
      default: return 'outline';
    }
  }

  // Keyboard shortcuts
  function handleKeydown(e: KeyboardEvent) {
    if (actionLoading || showRejectDialog) return;
    
    // A = Approve
    if (e.key === 'a' && location?.review_status !== 'approved') {
      e.preventDefault();
      updateStatus('approved');
    }
    // R = Reject (opens dialog)
    if (e.key === 'r' && location?.review_status !== 'rejected') {
      e.preventDefault();
      showRejectDialog = true;
    }
    // M = Open in Maps
    if (e.key === 'm') {
      e.preventDefault();
      openGoogleMaps();
    }
    // Escape = Go back
    if (e.key === 'Escape') {
      e.preventDefault();
      goto('/admin/locations');
    }
  }

  onMount(() => {
    loadLocation();
    // Add keyboard shortcuts
    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });
</script>

<svelte:head>
  <title>Location {locationId} - Admin - DGuesser</title>
</svelte:head>

<div class="space-y-6">
  <!-- Header -->
  <div class="flex items-center gap-4">
    <Button variant="ghost" size="sm" href="/admin/locations">
      <ArrowLeftIcon class="size-4 mr-2" />
      Back to Queue
    </Button>
  </div>

  {#if loading}
    <div class="grid gap-6 lg:grid-cols-2">
      <Skeleton class="h-[400px] rounded-lg" />
      <div class="space-y-4">
        <Skeleton class="h-10 w-48" />
        <Skeleton class="h-24" />
        <Skeleton class="h-32" />
      </div>
    </div>
  {:else if !location}
    <div class="text-center py-12">
      <p class="text-muted-foreground">Location not found</p>
    </div>
  {:else}
    <div class="grid gap-6 lg:grid-cols-2">
      <!-- Street View Preview -->
      <div class="space-y-4">
        <Card.Root>
          <Card.Content class="p-0">
            <img
              src="https://maps.googleapis.com/maps/api/streetview?size=800x500&pano={location.panorama_id}&key={import.meta.env.VITE_GOOGLE_MAPS_API_KEY}"
              alt="Street View"
              class="w-full h-[400px] object-cover rounded-lg"
            />
          </Card.Content>
        </Card.Root>

        <!-- Action Buttons -->
        <div class="text-xs text-muted-foreground mb-2 flex items-center gap-4">
          <span>Keyboard shortcuts:</span>
          <span><kbd class="px-1 py-0.5 bg-muted rounded text-[10px]">A</kbd> Approve</span>
          <span><kbd class="px-1 py-0.5 bg-muted rounded text-[10px]">R</kbd> Reject</span>
          <span><kbd class="px-1 py-0.5 bg-muted rounded text-[10px]">M</kbd> Maps</span>
          <span><kbd class="px-1 py-0.5 bg-muted rounded text-[10px]">Esc</kbd> Back</span>
        </div>
        <div class="flex gap-2">
          <Button onclick={openGoogleMaps} variant="outline" class="flex-1">
            <ExternalLinkIcon class="size-4 mr-2" />
            Open in Maps
          </Button>
          <Button
            onclick={() => updateStatus('approved')}
            disabled={actionLoading || location.review_status === 'approved'}
            class="flex-1 bg-green-600 hover:bg-green-700"
          >
            <CheckIcon class="size-4 mr-2" />
            Approve
          </Button>
          <Button
            onclick={() => showRejectDialog = true}
            disabled={actionLoading || location.review_status === 'rejected'}
            variant="destructive"
            class="flex-1"
          >
            <XIcon class="size-4 mr-2" />
            Reject
          </Button>
        </div>
      </div>

      <!-- Location Details -->
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <h1 class="text-2xl font-bold font-mono">{location.id}</h1>
          <Badge variant={getStatusBadgeVariant(location.review_status)}>
            {location.review_status}
          </Badge>
        </div>

        <!-- Status Card -->
        {#if location.failure_count > 0}
          <Card.Root class="border-amber-200 bg-amber-50 dark:bg-amber-950/20 dark:border-amber-800">
            <Card.Content class="flex items-center gap-3 p-4">
              <AlertTriangleIcon class="size-5 text-amber-600" />
              <div>
                <p class="font-medium text-amber-800 dark:text-amber-200">
                  {location.failure_count} report{location.failure_count !== 1 ? 's' : ''}
                </p>
                {#if location.last_failure_reason}
                  <p class="text-sm text-amber-700 dark:text-amber-300">
                    Last reason: {location.last_failure_reason.replace('_', ' ')}
                  </p>
                {/if}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Metadata -->
        <Card.Root>
          <Card.Header>
            <Card.Title>Location Details</Card.Title>
          </Card.Header>
          <Card.Content class="space-y-3">
            <div class="grid grid-cols-2 gap-4 text-sm">
              <div>
                <p class="text-muted-foreground">Coordinates</p>
                <p class="font-mono">{location.lat.toFixed(6)}, {location.lng.toFixed(6)}</p>
              </div>
              <div>
                <p class="text-muted-foreground">Country</p>
                <p>{location.country_code || 'Unknown'}</p>
              </div>
              <div>
                <p class="text-muted-foreground">Provider</p>
                <p class="capitalize">{location.provider.replace('_', ' ')}</p>
              </div>
              <div>
                <p class="text-muted-foreground">Source</p>
                <p class="capitalize">{location.source}</p>
              </div>
              {#if location.capture_date}
                <div>
                  <p class="text-muted-foreground">Capture Date</p>
                  <p>{location.capture_date}</p>
                </div>
              {/if}
              {#if location.elevation}
                <div>
                  <p class="text-muted-foreground">Elevation</p>
                  <p>{location.elevation}m</p>
                </div>
              {/if}
              {#if location.surface}
                <div>
                  <p class="text-muted-foreground">Surface</p>
                  <p class="capitalize">{location.surface}</p>
                </div>
              {/if}
              <div>
                <p class="text-muted-foreground">Active</p>
                <p>{location.active ? 'Yes' : 'No'}</p>
              </div>
            </div>
          </Card.Content>
        </Card.Root>

        <!-- Review History -->
        {#if location.reviewed_at}
          <Card.Root>
            <Card.Header>
              <Card.Title>Review History</Card.Title>
            </Card.Header>
            <Card.Content>
              <p class="text-sm text-muted-foreground">
                Last reviewed on {new Date(location.reviewed_at).toLocaleDateString()}
                {#if location.reviewed_by}
                  by {location.reviewed_by}
                {/if}
              </p>
            </Card.Content>
          </Card.Root>
        {/if}

        <!-- Reports -->
        {#if location.reports.length > 0}
          <Card.Root>
            <Card.Header>
              <Card.Title>Reports ({location.reports.length})</Card.Title>
            </Card.Header>
            <Card.Content>
              <div class="space-y-3 max-h-64 overflow-y-auto">
                {#each location.reports as report}
                  <div class="flex items-start gap-3 p-3 rounded-lg bg-muted/50">
                    <FlagIcon class="size-4 text-muted-foreground mt-0.5" />
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <Badge variant="outline" class="capitalize">
                          {report.reason.replace('_', ' ')}
                        </Badge>
                        <span class="text-xs text-muted-foreground">
                          {new Date(report.created_at).toLocaleDateString()}
                        </span>
                      </div>
                      {#if report.notes}
                        <p class="text-sm text-muted-foreground mt-1">{report.notes}</p>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            </Card.Content>
          </Card.Root>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Reject Confirmation Dialog -->
<AlertDialog.Root bind:open={showRejectDialog}>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>Reject Location</AlertDialog.Title>
      <AlertDialog.Description>
        Are you sure you want to reject this location? This will deactivate it and remove it from gameplay.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
      <AlertDialog.Action onclick={() => updateStatus('rejected')} class="bg-destructive text-destructive-foreground hover:bg-destructive/90">
        Reject Location
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
