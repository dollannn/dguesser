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

/** Game phase for reconnection logic */
export type GamePhase = 'lobby' | 'active' | null;

export interface SocketState {
  status: ConnectionStatus;
  error: string | null;
  reconnectAttempt: number;
  maxReconnectAttempts: number;
  /** Game ID to rejoin on reconnect (if any) */
  activeGameId: string | null;
  /** Game phase - used to determine if auto-rejoin is allowed */
  activeGamePhase: GamePhase;
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
  /** Listeners registered before socket was created */
  private pendingListeners: Array<{ event: string; callback: (data: unknown) => void }> = [];

  constructor() {
    this.state = writable({
      status: 'disconnected',
      error: null,
      reconnectAttempt: 0,
      maxReconnectAttempts: RECONNECTION_CONFIG.reconnectionAttempts,
      activeGameId: null,
      activeGamePhase: null,
    });
  }

  connect(): void {
    // Already connected - nothing to do
    if (this.socket?.connected) return;

    // Socket exists but disconnected - just reconnect it
    if (this.socket) {
      this.state.update((s) => ({ ...s, status: 'connecting', error: null }));
      this.socket.connect();
      return;
    }

    // No socket exists - create a new one
    this.state.update((s) => ({ ...s, status: 'connecting', error: null }));

    this.socket = io(REALTIME_URL, {
      transports: ['websocket'],
      autoConnect: true,
      withCredentials: true,
      ...RECONNECTION_CONFIG,
    });

    this.setupEventHandlers();

    // Attach any listeners that were registered before socket was created
    for (const { event, callback } of this.pendingListeners) {
      this.socket.on(event, callback);
    }
    this.pendingListeners = [];
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

      // Auto-rejoin active game ONLY if it was in active phase (not lobby)
      // In lobby phase, disconnection should require manual rejoin
      const { activeGameId, activeGamePhase } = get(this.state);
      if (activeGameId && activeGamePhase === 'active') {
        this.emit('game:join', { game_id: activeGameId });
      } else if (activeGameId && activeGamePhase === 'lobby') {
        // Clear the active game - user must manually rejoin lobby
        this.state.update((s) => ({ ...s, activeGameId: null, activeGamePhase: null }));
        toastStore.add('info', 'You were disconnected from the lobby. Please rejoin.');
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

  /** Set the active game ID and phase (for auto-rejoin on reconnect) */
  setActiveGame(gameId: string | null, phase: GamePhase = null): void {
    this.state.update((s) => ({ ...s, activeGameId: gameId, activeGamePhase: phase }));
  }

  /** Update just the game phase (e.g., when game starts) */
  setActiveGamePhase(phase: GamePhase): void {
    this.state.update((s) => ({ ...s, activeGamePhase: phase }));
  }

  /**
   * Wait for the socket to be authenticated.
   * Resolves immediately if already authenticated, otherwise waits for auth:success.
   * Will initiate connection if not already connecting/connected.
   * Rejects on auth:error or timeout.
   */
  async waitForAuth(timeoutMs = 10000): Promise<void> {
    const currentStatus = get(this.state).status;
    if (currentStatus === 'authenticated') return;

    // If socket not connected and not connecting, start connection
    if (!this.socket?.connected && currentStatus !== 'connecting') {
      this.connect();
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        cleanup();
        reject(new Error('Authentication timeout'));
      }, timeoutMs);

      const cleanup = () => {
        clearTimeout(timeout);
        this.socket?.off('auth:success', onSuccess);
        this.socket?.off('auth:error', onError);
        this.socket?.off('connect_error', onConnectError);
      };

      const onSuccess = () => {
        cleanup();
        resolve();
      };

      const onError = (data: { error: string }) => {
        cleanup();
        reject(new Error(data.error));
      };

      const onConnectError = (error: Error) => {
        cleanup();
        reject(new Error(`Connection failed: ${error.message}`));
      };

      // Check again in case status changed during setup
      if (get(this.state).status === 'authenticated') {
        cleanup();
        resolve();
        return;
      }

      this.socket?.once('auth:success', onSuccess);
      this.socket?.once('auth:error', onError);
      this.socket?.once('connect_error', onConnectError);
    });
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
    this.pendingListeners = [];
    this.state.set({
      status: 'disconnected',
      error: null,
      reconnectAttempt: 0,
      maxReconnectAttempts: RECONNECTION_CONFIG.reconnectionAttempts,
      activeGameId: null,
      activeGamePhase: null,
    });
  }

  emit(event: string, data?: unknown): void {
    this.socket?.emit(event, data);
  }

  on<T>(event: string, callback: (data: T) => void): () => void {
    const wrappedCallback = callback as (data: unknown) => void;

    if (this.socket) {
      this.socket.on(event, callback);
    } else {
      // Buffer listener to be attached when socket is created
      this.pendingListeners.push({ event, callback: wrappedCallback });
    }

    return () => {
      this.socket?.off(event, callback);
      // Also remove from pending if socket not yet created
      this.pendingListeners = this.pendingListeners.filter(
        (l) => !(l.event === event && l.callback === wrappedCallback)
      );
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
