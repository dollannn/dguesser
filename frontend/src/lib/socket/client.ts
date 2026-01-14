import { io, Socket } from 'socket.io-client';
import { writable, type Writable } from 'svelte/store';

const REALTIME_URL = import.meta.env.VITE_REALTIME_URL || 'http://localhost:3002';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'authenticated';

interface SocketState {
  status: ConnectionStatus;
  error: string | null;
}

class SocketClient {
  private socket: Socket | null = null;
  public state: Writable<SocketState>;

  constructor() {
    this.state = writable({
      status: 'disconnected',
      error: null,
    });
  }

  connect(): void {
    if (this.socket?.connected) return;

    this.state.update((s) => ({ ...s, status: 'connecting' }));

    this.socket = io(REALTIME_URL, {
      transports: ['websocket'],
      autoConnect: true,
      withCredentials: true,
    });

    this.socket.on('connect', () => {
      this.state.update((s) => ({ ...s, status: 'connected', error: null }));
      // Auto-authenticate using session cookie
      this.authenticate();
    });

    this.socket.on('disconnect', (reason) => {
      this.state.update((s) => ({
        ...s,
        status: 'disconnected',
        error: reason === 'io server disconnect' ? 'Server disconnected' : null,
      }));
    });

    this.socket.on('connect_error', (error) => {
      this.state.update((s) => ({
        ...s,
        status: 'disconnected',
        error: error.message,
      }));
    });

    this.socket.on('auth:success', () => {
      this.state.update((s) => ({ ...s, status: 'authenticated' }));
    });

    this.socket.on('auth:error', (data: { error: string }) => {
      this.state.update((s) => ({ ...s, error: data.error }));
    });
  }

  private authenticate(): void {
    // Session ID is sent via cookie, just trigger auth
    this.socket?.emit('auth', { session_id: '' });
  }

  disconnect(): void {
    this.socket?.disconnect();
    this.socket = null;
    this.state.set({ status: 'disconnected', error: null });
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
}

export const socketClient = new SocketClient();
