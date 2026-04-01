import { writable } from 'svelte/store';

/** Toast notification types */
export type ToastType = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}

/** Toast store for connection notifications */
function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);

  let toastId = 0;

  return {
    subscribe,
    add(type: ToastType, message: string, duration = 4000): void {
      const id = `toast-${++toastId}`;
      update((toasts) => [...toasts, { id, type, message, duration }]);

      if (duration > 0) {
        setTimeout(() => {
          update((toasts) => toasts.filter((t) => t.id !== id));
        }, duration);
      }
    },
    remove(id: string): void {
      update((toasts) => toasts.filter((t) => t.id !== id));
    },
    clear(): void {
      update(() => []);
    },
  };
}

export const toastStore = createToastStore();
