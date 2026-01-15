import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };

// =============================================================================
// Rank Display Utilities
// =============================================================================

/**
 * Get the display string for a rank (1st, 2nd, 3rd, #4, etc.)
 */
export function getRankDisplay(rank: number): string {
  switch (rank) {
    case 1:
      return '1st';
    case 2:
      return '2nd';
    case 3:
      return '3rd';
    default:
      return `#${rank}`;
  }
}

/**
 * Get Tailwind classes for rank styling
 */
export function getRankClass(rank: number): string {
  switch (rank) {
    case 1:
      return 'text-yellow-500 font-bold';
    case 2:
      return 'text-gray-400 font-bold';
    case 3:
      return 'text-amber-600 font-bold';
    default:
      return 'text-gray-500';
  }
}

/**
 * Get background classes for rank rows (for leaderboard tables)
 */
export function getRankRowClass(rank: number, isCurrentUser: boolean = false): string {
  const base = isCurrentUser ? 'bg-primary-50 ring-2 ring-primary-200 ring-inset' : '';

  switch (rank) {
    case 1:
      return isCurrentUser ? base : 'bg-yellow-50/50';
    case 2:
      return isCurrentUser ? base : 'bg-gray-50/50';
    case 3:
      return isCurrentUser ? base : 'bg-amber-50/50';
    default:
      return base;
  }
}

/**
 * Format a score number with locale-aware thousands separators
 */
export function formatScore(score: number): string {
  return score.toLocaleString();
}

/**
 * Format distance in meters to human-readable form
 * Returns "No guess" for negative values (sentinel for timeout with no guess)
 */
export function formatDistance(meters: number): string {
  if (meters < 0) {
    return 'No guess';
  }
  if (meters < 1000) {
    return `${Math.round(meters)} m`;
  }
  return `${(meters / 1000).toFixed(1)} km`;
}
