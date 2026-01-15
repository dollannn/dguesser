import { writable, derived } from 'svelte/store';
import { authApi, type User } from '$lib/api/auth';
import { browser } from '$app/environment';

interface AuthState {
  user: User | null;
  loading: boolean;
  initialized: boolean;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>({
    user: null,
    loading: true,
    initialized: false,
  });

  return {
    subscribe,

    async initialize(): Promise<void> {
      if (!browser) return;

      update((s) => ({ ...s, loading: true }));

      try {
        const user = await authApi.getCurrentUser();
        set({ user, loading: false, initialized: true });
      } catch {
        // No valid session - that's okay
        set({ user: null, loading: false, initialized: true });
      }
    },

    async createGuest(): Promise<User> {
      update((s) => ({ ...s, loading: true }));
      
      try {
        const user = await authApi.createGuest();
        set({ user, loading: false, initialized: true });
        return user;
      } catch (error) {
        update((s) => ({ ...s, loading: false }));
        throw error;
      }
    },

    async logout(): Promise<void> {
      try {
        await authApi.logout();
      } finally {
        set({ user: null, loading: false, initialized: true });
      }
    },

    setUser(user: User): void {
      set({ user, loading: false, initialized: true });
    },
  };
}

export const authStore = createAuthStore();

// Derived stores for convenience
export const user = derived(authStore, ($auth) => $auth.user);
export const isAuthenticated = derived(authStore, ($auth) => $auth.user !== null);
export const isGuest = derived(authStore, ($auth) => $auth.user?.is_guest ?? true);
export const isLoading = derived(authStore, ($auth) => $auth.loading);
export const isAdmin = derived(authStore, ($auth) => $auth.user?.role === 'admin');
