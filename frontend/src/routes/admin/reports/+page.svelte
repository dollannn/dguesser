<script lang="ts">
  import { onMount } from 'svelte';
  import { adminApi, type ReportsListResponse } from '$lib/api/admin';
  import { toast } from 'svelte-sonner';
  import * as Table from '$lib/components/ui/table';
  import { Button } from '$lib/components/ui/button';
  import { Badge } from '$lib/components/ui/badge';
  import { Skeleton } from '$lib/components/ui/skeleton';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
  import EyeIcon from '@lucide/svelte/icons/eye';
  import FlagIcon from '@lucide/svelte/icons/flag';
  import SEO from '$lib/components/SEO.svelte';

  let data: ReportsListResponse | null = $state(null);
  let loading = $state(true);
  let page = $state(1);
  let reasonFilter = $state<string | undefined>(undefined);
  let statusFilter = $state<string | undefined>(undefined);

  async function loadData() {
    loading = true;
    try {
      data = await adminApi.getReports({ 
        page, 
        reason: reasonFilter, 
        location_status: statusFilter 
      });
    } catch (e) {
      toast.error('Failed to load reports');
      console.error('Failed to load reports:', e);
    } finally {
      loading = false;
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

  function getReasonBadgeVariant(reason: string): 'default' | 'secondary' | 'destructive' | 'outline' {
    switch (reason) {
      case 'zero_results': return 'destructive';
      case 'corrupted': return 'destructive';
      case 'indoor': return 'secondary';
      default: return 'outline';
    }
  }

  function openGoogleMaps(lat: number, lng: number) {
    window.open(`https://www.google.com/maps?q=${lat},${lng}`, '_blank');
  }

  onMount(() => {
    loadData();
  });

  // Reload when page or filters change
  $effect(() => {
    page;
    reasonFilter;
    statusFilter;
    loadData();
  });
</script>

<SEO title="Admin Reports" noindex />

<div class="space-y-6">
  <!-- Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
    <div>
      <h1 class="text-3xl font-bold">Reports</h1>
      <p class="text-muted-foreground">
        View all user-submitted location reports
      </p>
    </div>
    
    <div class="flex items-center gap-2">
      <select
        class="h-9 w-40 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus:outline-none focus:ring-2 focus:ring-ring"
        onchange={(e) => {
          const value = e.currentTarget.value;
          reasonFilter = value === 'all' ? undefined : value;
          page = 1;
        }}
      >
        <option value="all">All Reasons</option>
        <option value="zero_results">Zero Results</option>
        <option value="corrupted">Corrupted</option>
        <option value="low_quality">Low Quality</option>
        <option value="indoor">Indoor</option>
        <option value="restricted">Restricted</option>
        <option value="other">Other</option>
      </select>

      <select
        class="h-9 w-40 rounded-md border border-input bg-background px-3 py-1 text-sm shadow-sm focus:outline-none focus:ring-2 focus:ring-ring"
        onchange={(e) => {
          const value = e.currentTarget.value;
          statusFilter = value === 'all' ? undefined : value;
          page = 1;
        }}
      >
        <option value="all">All Statuses</option>
        <option value="approved">Approved</option>
        <option value="flagged">Flagged</option>
        <option value="pending">Pending</option>
        <option value="rejected">Rejected</option>
      </select>
    </div>
  </div>

  <!-- Table -->
  <div class="border rounded-lg">
    <Table.Root>
      <Table.Header>
        <Table.Row>
          <Table.Head class="w-24">Preview</Table.Head>
          <Table.Head>Location</Table.Head>
          <Table.Head>Reason</Table.Head>
          <Table.Head>Status</Table.Head>
          <Table.Head>Date</Table.Head>
          <Table.Head class="text-right">Actions</Table.Head>
        </Table.Row>
      </Table.Header>
      <Table.Body>
        {#if loading}
          {#each Array(5) as _}
            <Table.Row>
              <Table.Cell><Skeleton class="h-12 w-20 rounded" /></Table.Cell>
              <Table.Cell><Skeleton class="h-4 w-32" /></Table.Cell>
              <Table.Cell><Skeleton class="h-6 w-24 rounded-full" /></Table.Cell>
              <Table.Cell><Skeleton class="h-6 w-20 rounded-full" /></Table.Cell>
              <Table.Cell><Skeleton class="h-4 w-24" /></Table.Cell>
              <Table.Cell><Skeleton class="h-8 w-20 ml-auto" /></Table.Cell>
            </Table.Row>
          {/each}
        {:else if !data?.reports.length}
          <Table.Row>
            <Table.Cell colspan={6} class="h-32 text-center">
              <div class="flex flex-col items-center gap-2 text-muted-foreground">
                <FlagIcon class="size-8" />
                <p>No reports found</p>
              </div>
            </Table.Cell>
          </Table.Row>
        {:else}
          {#each data.reports as report}
            <Table.Row>
              <!-- Thumbnail -->
              <Table.Cell>
                <a href="/admin/locations/{report.location_id}" class="block">
                  <img
                    src="https://maps.googleapis.com/maps/api/streetview?size=100x60&pano={report.panorama_id}&key={import.meta.env.VITE_GOOGLE_MAPS_API_KEY}"
                    alt="Street View preview"
                    class="rounded border object-cover w-20 h-12 hover:opacity-80 transition-opacity"
                  />
                </a>
              </Table.Cell>

              <!-- Location Info -->
              <Table.Cell>
                <div class="space-y-1">
                  <a 
                    href="/admin/locations/{report.location_id}" 
                    class="font-mono text-sm hover:underline"
                  >
                    {report.location_id}
                  </a>
                  <div class="text-xs text-muted-foreground">
                    {report.lat.toFixed(4)}, {report.lng.toFixed(4)}
                    {#if report.country_code}
                      <span class="ml-1">({report.country_code})</span>
                    {/if}
                  </div>
                </div>
              </Table.Cell>

              <!-- Reason -->
              <Table.Cell>
                <Badge variant={getReasonBadgeVariant(report.reason)} class="capitalize">
                  {report.reason.replace('_', ' ')}
                </Badge>
                {#if report.notes}
                  <p class="text-xs text-muted-foreground mt-1 max-w-32 truncate" title={report.notes}>
                    {report.notes}
                  </p>
                {/if}
              </Table.Cell>

              <!-- Location Status -->
              <Table.Cell>
                <Badge variant={getStatusBadgeVariant(report.location_review_status)}>
                  {report.location_review_status}
                </Badge>
              </Table.Cell>

              <!-- Date -->
              <Table.Cell>
                <span class="text-sm text-muted-foreground">
                  {new Date(report.created_at).toLocaleDateString()}
                </span>
              </Table.Cell>

              <!-- Actions -->
              <Table.Cell class="text-right">
                <div class="flex items-center justify-end gap-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onclick={() => openGoogleMaps(report.lat, report.lng)}
                    title="Open in Google Maps"
                  >
                    <ExternalLinkIcon class="size-4" />
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    href="/admin/locations/{report.location_id}"
                    title="View Location"
                  >
                    <EyeIcon class="size-4" />
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
        Showing {((page - 1) * data.per_page) + 1} - {Math.min(page * data.per_page, data.total)} of {data.total} reports
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
