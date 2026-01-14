import { io, Socket } from 'socket.io-client';
import { writable, get, type Writable } from 'svelte/store';

const REALTIME_URL = import.meta.env.VITE_REALTIME_URL || 'http://localhost:3002';

/** Reconnection configuration */
const RECONNECTION_CONFIG = {
  reconnection: true,
  reconnectionAttempts: 10,
  reconnectionDelay: 1000,
  reconnectionDelayMax: 5000,
  randomizationFactor: 0.5,
  timeout: 20000,
} as const;

export type ConnectionStatus =
  | 'disconnected'
  | 'connecting'
  | 'connected'
  | 'authenticated'
  | 'reconnecting';

export interface SocketState {
  status: ConnectionStatus;
  error: string | null;
  reconnectAttempt: number;
  maxReconnectAttempts: number;
  /** Game ID to rejoin on reconnect (if any) */
  activeGameId: string | null;
}

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

class SocketClient {
  private socket: Socket | null = null;
  public state: Writable<SocketState>;
  private reconnectCallbacks: Array<() => void> = [];

  constructor() {
    this.state = writable({
      status: 'disconnected',
      error: null,
      reconnectAttempt: 0,
      maxReconnectAttempts: RECONNECTION_CONFIG.reconnectionAttempts,
      activeGameId: null,
    });
  }

  connect(): void {
    if (this.socket?.connected) return;

    this.state.update((s) => ({ ...s, status: 'connecting', error: null }));

    this.socket = io(REALTIME_URL, {
      transports: ['websocket'],
      autoConnect: true,
      withCredentials: true,
      ...RECONNECTION_CONFIG,
    });

    this.setupEventHandlers();
  }

  private setupEventHandlers(): void {
    if (!this.socket) return;

    // Connection established
    this.socket.on('connect', () => {
      const currentState = get(this.state);
      const wasReconnecting = currentState.status === 'reconnecting';

      this.state.update((s) => ({
        ...s,
        status: 'connected',
        error: null,
        reconnectAttempt: 0,
      }));

      // Auto-authenticate using session cookie
      this.authenticate();

      if (wasReconnecting) {
        toastStore.add('success', 'Reconnected!');
      }
    });

    // Disconnection
    this.socket.on('disconnect', (reason) => {
      const isServerDisconnect = reason === 'io server disconnect';
      const isTransportClose = reason === 'transport close';

      this.state.update((s) => ({
        ...s,
        status: 'disconnected',
        error: isServerDisconnect ? 'Server disconnected' : null,
      }));

      // Show toast for unexpected disconnections
      if (isTransportClose || isServerDisconnect) {
        toastStore.add('warning', 'Connection lost. Reconnecting...');
      }

      // If server explicitly disconnected us, we might need to manually reconnect
      if (isServerDisconnect && this.socket) {
        this.socket.connect();
      }
    });

    // Connection error
    this.socket.on('connect_error', (error) => {
      this.state.update((s) => ({
        ...s,
        status: 'disconnected',
        error: error.message,
      }));
    });

    // Reconnection attempt starting
    this.socket.io.on('reconnect_attempt', (attempt) => {
      this.state.update((s) => ({
        ...s,
        status: 'reconnecting',
        reconnectAttempt: attempt,
      }));
    });

    // Reconnection successful
    this.socket.io.on('reconnect', (_attempt) => {
      this.state.update((s) => ({
        ...s,
        status: 'connected',
        reconnectAttempt: 0,
        error: null,
      }));

      // Execute any registered reconnect callbacks
      this.reconnectCallbacks.forEach((cb) => cb());
    });

    // Reconnection error (single attempt failed)
    this.socket.io.on('reconnect_error', (_error) => {
      // State is already 'reconnecting', just let it continue
    });

    // Reconnection failed after all attempts
    this.socket.io.on('reconnect_failed', () => {
      this.state.update((s) => ({
        ...s,
        status: 'disconnected',
        reconnectAttempt: 0,
        error: 'Failed to reconnect after multiple attempts',
      }));
      toastStore.add('error', 'Failed to reconnect. Please refresh the page.', 0);
    });

    // Authentication success
    this.socket.on('auth:success', () => {
      this.state.update((s) => ({ ...s, status: 'authenticated' }));

      // Rejoin active game if we were in one
      const { activeGameId } = get(this.state);
      if (activeGameId) {
        this.emit('game:join', { game_id: activeGameId });
      }
    });

    // Authentication error
    this.socket.on('auth:error', (data: { error: string }) => {
      this.state.update((s) => ({ ...s, error: data.error }));
    });
  }

  private authenticate(): void {
    // Session ID is sent via cookie, just trigger auth
    this.socket?.emit('auth', { session_id: '' });
  }

  /** Set the active game ID (for auto-rejoin on reconnect) */
  setActiveGame(gameId: string | null): void {
    this.state.update((s) => ({ ...s, activeGameId: gameId }));
  }

  /** Register a callback to run on successful reconnection */
  onReconnect(callback: () => void): () => void {
    this.reconnectCallbacks.push(callback);
    return () => {
      this.reconnectCallbacks = this.reconnectCallbacks.filter((cb) => cb !== callback);
    };
  }

  /** Manually trigger reconnection */
  reconnect(): void {
    if (this.socket) {
      this.socket.connect();
    } else {
      this.connect();
    }
  }

  disconnect(): void {
    this.socket?.disconnect();
    this.socket = null;
    this.state.set({
      status: 'disconnected',
      error: null,
      reconnectAttempt: 0,
      maxReconnectAttempts: RECONNECTION_CONFIG.reconnectionAttempts,
      activeGameId: null,
    });
  }

  emit(event: string, data?: unknown): void {
    this.socket?.emit(event, data);
  }

  on<T>(event: string, callback: (data: T) => void): () => void {
    this.socket?.on(event, callback);
    return () => {
      this.socket?.off(event, callback);
    };
  }

  once<T>(event: string, callback: (data: T) => void): void {
    this.socket?.once(event, callback);
  }

  off(event: string): void {
    this.socket?.off(event);
  }

  get connected(): boolean {
    return this.socket?.connected ?? false;
  }

  get socketId(): string | undefined {
    return this.socket?.id;
  }
}

export const socketClient = new SocketClient();
