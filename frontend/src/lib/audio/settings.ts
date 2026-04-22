import { browser } from '$app/environment';
import { writable } from 'svelte/store';

const STORAGE_KEY = 'dguesser:sound-settings';
const DEFAULT_VOLUME = 0.35;

export interface SoundSettingsState {
  enabled: boolean;
  volume: number;
  unlocked: boolean;
  initialized: boolean;
}

const initialState: SoundSettingsState = {
  enabled: true,
  volume: DEFAULT_VOLUME,
  unlocked: false,
  initialized: false,
};

function clampVolume(volume: number): number {
  return Math.max(0, Math.min(1, volume));
}

function createSoundSettingsStore() {
  const { subscribe, set, update } = writable<SoundSettingsState>(initialState);

  let hasInitialized = false;

  function persist(state: SoundSettingsState): void {
    if (!browser) return;

    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        enabled: state.enabled,
        volume: state.volume,
      }),
    );
  }

  return {
    subscribe,

    initialize(): void {
      if (!browser || hasInitialized) {
        return;
      }

      hasInitialized = true;

      try {
        const raw = localStorage.getItem(STORAGE_KEY);
        if (!raw) {
          set({ ...initialState, initialized: true });
          return;
        }

        const parsed = JSON.parse(raw) as Partial<Pick<SoundSettingsState, 'enabled' | 'volume'>>;
        set({
          enabled: parsed.enabled ?? initialState.enabled,
          volume: clampVolume(parsed.volume ?? initialState.volume),
          unlocked: false,
          initialized: true,
        });
      } catch {
        set({ ...initialState, initialized: true });
      }
    },

    setEnabled(enabled: boolean): void {
      update((state) => {
        const next = { ...state, enabled };
        persist(next);
        return next;
      });
    },

    toggleEnabled(): void {
      update((state) => {
        const next = { ...state, enabled: !state.enabled };
        persist(next);
        return next;
      });
    },

    setVolume(volume: number): void {
      update((state) => {
        const next = { ...state, volume: clampVolume(volume) };
        persist(next);
        return next;
      });
    },

    markUnlocked(): void {
      update((state) => (state.unlocked ? state : { ...state, unlocked: true }));
    },

    reset(): void {
      hasInitialized = false;
      set(initialState);
    },
  };
}

export const soundSettings = createSoundSettingsStore();
