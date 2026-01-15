<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { authStore, isAdmin, isLoading } from '$lib/stores/auth';
  import { Spinner } from '$lib/components/ui/spinner';
  import { Button } from '$lib/components/ui/button';
  import { Separator } from '$lib/components/ui/separator';
  import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
  import MapPinIcon from '@lucide/svelte/icons/map-pin';
  import FlagIcon from '@lucide/svelte/icons/flag';
  import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';
  import ShieldAlertIcon from '@lucide/svelte/icons/shield-alert';

  let { children } = $props();

  // Redirect non-admins once auth is loaded
  $effect(() => {
    if (!$isLoading && !$isAdmin) {
      goto('/');
    }
  });

  const navItems = [
    { href: '/admin', label: 'Dashboard', icon: LayoutDashboardIcon },
    { href: '/admin/locations', label: 'Review Queue', icon: MapPinIcon },
    { href: '/admin/reports', label: 'Reports', icon: FlagIcon },
  ];

  function isActive(href: string): boolean {
    const pathname = $page.url.pathname;
    if (href === '/admin') {
      return pathname === '/admin';
    }
    return pathname.startsWith(href);
  }
</script>

{#if $isLoading}
  <div class="flex items-center justify-center h-64">
    <Spinner class="size-10 text-primary" />
  </div>
{:else if !$isAdmin}
  <div class="flex flex-col items-center justify-center h-64 gap-4">
    <ShieldAlertIcon class="size-16 text-muted-foreground" />
    <h1 class="text-2xl font-bold">Access Denied</h1>
    <p class="text-muted-foreground">You don't have permission to access this page.</p>
    <Button href="/" variant="outline">
      <ArrowLeftIcon class="size-4 mr-2" />
      Go Home
    </Button>
  </div>
{:else}
  <div class="flex min-h-[calc(100vh-4rem)]">
    <!-- Sidebar -->
    <aside class="w-64 border-r bg-muted/30 flex-shrink-0">
      <div class="p-4">
        <div class="flex items-center gap-2 mb-6">
          <ShieldAlertIcon class="size-5 text-primary" />
          <h2 class="font-semibold text-lg">Admin Panel</h2>
        </div>
        
        <nav class="space-y-1">
          {#each navItems as item}
            <a
              href={item.href}
              class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors
                {isActive(item.href)
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted'}"
            >
              <item.icon class="size-4" />
              {item.label}
            </a>
          {/each}
        </nav>
        
        <Separator class="my-6" />
        
        <Button href="/" variant="ghost" class="w-full justify-start">
          <ArrowLeftIcon class="size-4 mr-2" />
          Back to Site
        </Button>
      </div>
    </aside>

    <!-- Main content -->
    <main class="flex-1 p-6 overflow-auto">
      {@render children()}
    </main>
  </div>
{/if}
