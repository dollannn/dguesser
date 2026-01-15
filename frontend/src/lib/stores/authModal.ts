import { writable } from 'svelte/store';

function createAuthModalStore() {
  const { subscribe, set, update } = writable(false);

  return {
    subscribe,
    set,
    open: () => set(true),
    close: () => set(false),
    toggle: () => update((open) => !open),
  };
}

export const authModalOpen = createAuthModalStore();
