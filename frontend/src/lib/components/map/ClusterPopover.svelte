<script lang="ts">
  /**
   * Floating list popover used to reveal members of a large cluster (>=6) or
   * during the desktop hover-preview of any cluster.
   *
   * Positioned in container-pixel space relative to its parent (the results
   * map wrapper). The parent is expected to be `position: relative` so the
   * popover's absolute coordinates resolve against the map viewport.
   *
   * The popover is keyboard accessible:
   *   - Up/Down arrows navigate members
   *   - Enter / Space selects / keeps the member highlighted
   *   - Escape closes (delegated to parent)
   */
  import { onMount, onDestroy } from 'svelte';
  import { formatDistance } from '$lib/utils';

  export interface PopoverMember {
    userId: string;
    displayName: string;
    distanceMeters: number;
    color: string;
    isCurrentUser: boolean;
  }

  interface Props {
    /** Container-pixel x of the cluster centroid. */
    x: number;
    /** Container-pixel y of the cluster centroid. */
    y: number;
    /** Container (map viewport) dimensions for edge clamping. */
    containerWidth: number;
    containerHeight: number;
    /** Members in display order (current user first). */
    members: PopoverMember[];
    /** Whether the popover is in non-sticky hover-preview mode. */
    hoverPreview?: boolean;
    /** Fires when the pointer hovers or keyboard focus moves to a member. */
    onMemberHover?: (userId: string | null) => void;
    /** Fires when the user clicks a row (for analytics / selection). */
    onMemberSelect?: (userId: string) => void;
    /** Fires when the popover requests close (outside click, Esc, etc). */
    onClose?: () => void;
    /** Fires when the pointer leaves the popover entirely. */
    onPointerLeave?: () => void;
  }

  let {
    x,
    y,
    containerWidth,
    containerHeight,
    members,
    hoverPreview = false,
    onMemberHover,
    onMemberSelect,
    onClose,
    onPointerLeave,
  }: Props = $props();

  let rootEl: HTMLDivElement | null = $state(null);
  let focusedIndex = $state(0);

  // Measured popover size (falls back to an estimate before first measurement).
  let measuredWidth = $state(220);
  let measuredHeight = $state(120);

  // Compute placement: prefer above the cluster, otherwise flip below. Clamp
  // to viewport horizontally with a small margin.
  const margin = 12;
  const anchorOffset = 18; // pixels of gap between token and popover

  let placement = $derived.by(() => {
    const width = measuredWidth;
    const height = measuredHeight;
    const spaceAbove = y - margin;
    const spaceBelow = containerHeight - y - margin;
    const preferAbove = spaceAbove >= height + anchorOffset;
    const top = preferAbove
      ? y - height - anchorOffset
      : y + anchorOffset;
    const clampedTop = Math.max(
      margin,
      Math.min(containerHeight - height - margin, top),
    );
    let left = x - width / 2;
    left = Math.max(margin, Math.min(containerWidth - width - margin, left));
    return { top: clampedTop, left, side: preferAbove ? 'above' : 'below' as const };
  });

  function measure() {
    if (!rootEl) return;
    const rect = rootEl.getBoundingClientRect();
    if (rect.width > 0) measuredWidth = rect.width;
    if (rect.height > 0) measuredHeight = rect.height;
  }

  onMount(() => {
    measure();
    // Re-measure after the next frame in case initial fonts / styles change it.
    requestAnimationFrame(measure);
  });

  // ─── Keyboard navigation ─────────────────────────────────────────────
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose?.();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusedIndex = (focusedIndex + 1) % members.length;
      focusRow(focusedIndex);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusedIndex = (focusedIndex - 1 + members.length) % members.length;
      focusRow(focusedIndex);
    } else if (event.key === 'Home') {
      event.preventDefault();
      focusedIndex = 0;
      focusRow(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      focusedIndex = members.length - 1;
      focusRow(focusedIndex);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const member = members[focusedIndex];
      if (member) onMemberSelect?.(member.userId);
    }
  }

  function focusRow(index: number) {
    const row = rootEl?.querySelector<HTMLElement>(`[data-row-index="${index}"]`);
    row?.focus();
    const member = members[index];
    if (member) onMemberHover?.(member.userId);
  }

  // ─── Click-outside (sticky mode only) ────────────────────────────────
  function handleOutside(event: MouseEvent | PointerEvent) {
    if (!rootEl) return;
    const target = event.target as Element | null;
    if (!target) return;
    // Clicks on a cluster token are handled by the marker's click listener
    // (which toggles the popover). Ignoring them here prevents a race where
    // both handlers fire and the popover closes then re-opens.
    if (typeof target.closest === 'function' && target.closest('.cluster-token')) return;
    if (rootEl.contains(target)) return;
    onClose?.();
  }

  onMount(() => {
    if (hoverPreview) return; // hover preview is closed by parent grace-timer
    // Use a slight delay to avoid catching the click that opened the popover.
    const id = setTimeout(() => {
      document.addEventListener('pointerdown', handleOutside, true);
    }, 0);
    return () => {
      clearTimeout(id);
      document.removeEventListener('pointerdown', handleOutside, true);
    };
  });

  onDestroy(() => {
    // Nothing to do; highlight state is cleared by explicit mouseleave / close
    // events so the popover unmount doesn't drop an externally-driven hover.
  });

  function initial(name: string): string {
    const trimmed = name.trim();
    if (!trimmed) return '?';
    const codepoint = trimmed.codePointAt(0);
    if (codepoint === undefined) return '?';
    return String.fromCodePoint(codepoint).toUpperCase();
  }
</script>

<div
  bind:this={rootEl}
  class="cluster-popover"
  class:cluster-popover-preview={hoverPreview}
  class:cluster-popover-below={placement.side === 'below'}
  style="top: {placement.top}px; left: {placement.left}px;"
  role="dialog"
  tabindex="-1"
  aria-label="Overlapping players"
  onkeydown={handleKeydown}
  onmouseenter={() => {
    // Let the parent know we're still hovered so it doesn't close us.
    onMemberHover?.(members[focusedIndex]?.userId ?? null);
  }}
  onmouseleave={() => onPointerLeave?.()}
>
  <div class="cluster-popover-arrow" aria-hidden="true"></div>
  <ul class="cluster-popover-list">
    {#each members as member, i (member.userId || i)}
      <li>
        <button
          type="button"
          class="cluster-popover-row"
          class:is-you={member.isCurrentUser}
          data-row-index={i}
          tabindex={i === focusedIndex ? 0 : -1}
          onmouseenter={() => onMemberHover?.(member.userId)}
          onfocus={() => {
            focusedIndex = i;
            onMemberHover?.(member.userId);
          }}
          onclick={() => onMemberSelect?.(member.userId)}
        >
          <span
            class="cluster-popover-chip"
            style="background-color: {member.color}; color: #fff;"
            aria-hidden="true"
          >
            {initial(member.displayName)}
          </span>
          <span class="cluster-popover-name">
            {member.displayName}
            {#if member.isCurrentUser}
              <span class="cluster-popover-you">You</span>
            {/if}
          </span>
          <span class="cluster-popover-distance">
            {formatDistance(member.distanceMeters)}
          </span>
        </button>
      </li>
    {/each}
  </ul>
</div>
