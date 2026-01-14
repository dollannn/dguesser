// Svelte stores
// TODO: Implement in Phase 6

import { writable } from 'svelte/store';

// User store
export const user = writable<{ id: string; displayName: string; avatarUrl?: string } | null>(null);

// Game store
export const game = writable<{ id: string; code: string; players: unknown[] } | null>(null);
