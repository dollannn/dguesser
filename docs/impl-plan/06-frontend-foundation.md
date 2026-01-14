# Phase 6: Frontend Foundation

**Priority:** P0  
**Duration:** 3-4 days  
**Dependencies:** Phase 4 (API Crate)

## Objectives

- Set up SvelteKit project structure
- Configure Tailwind CSS
- Build API client with type safety
- Create Socket.IO client wrapper
- Implement authentication stores
- Build core layout and navigation
- **Use prefixed nanoid IDs in TypeScript types** (`usr_xxx`, `gam_xxx`, etc.)

## Deliverables

### 6.1 Project Setup

**frontend/package.json:**
```json
{
  "name": "dguesser-frontend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "check:watch": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json --watch",
    "lint": "eslint .",
    "format": "prettier --write ."
  },
  "devDependencies": {
    "@sveltejs/adapter-auto": "^3.0.0",
    "@sveltejs/kit": "^2.0.0",
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@types/node": "^22.0.0",
    "autoprefixer": "^10.4.0",
    "eslint": "^9.0.0",
    "postcss": "^8.4.0",
    "prettier": "^3.0.0",
    "prettier-plugin-svelte": "^3.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.0.0",
    "vite": "^6.0.0"
  },
  "dependencies": {
    "socket.io-client": "^4.7.0"
  },
  "type": "module"
}
```

**frontend/tailwind.config.js:**
```javascript
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#eff6ff',
          100: '#dbeafe',
          500: '#3b82f6',
          600: '#2563eb',
          700: '#1d4ed8',
        },
        accent: {
          500: '#22c55e',
          600: '#16a34a',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
```

**frontend/src/app.css:**
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  html {
    @apply antialiased;
  }
  
  body {
    @apply bg-gray-50 text-gray-900 min-h-screen;
  }
}

@layer components {
  .btn {
    @apply px-4 py-2 rounded-lg font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2;
  }
  
  .btn-primary {
    @apply btn bg-primary-600 text-white hover:bg-primary-700 focus:ring-primary-500;
  }
  
  .btn-secondary {
    @apply btn bg-gray-200 text-gray-800 hover:bg-gray-300 focus:ring-gray-500;
  }
  
  .btn-accent {
    @apply btn bg-accent-500 text-white hover:bg-accent-600 focus:ring-accent-500;
  }
  
  .card {
    @apply bg-white rounded-xl shadow-sm border border-gray-100 p-6;
  }
  
  .input {
    @apply w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent;
  }
}
```

### 6.2 API Client

**frontend/src/lib/api/client.ts:**
```typescript
const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3001';

interface ApiError {
  code: string;
  message: string;
}

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };

    const options: RequestInit = {
      method,
      headers,
      credentials: 'include', // Send cookies
    };

    if (body) {
      options.body = JSON.stringify(body);
    }

    const response = await fetch(url, options);

    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        code: 'UNKNOWN',
        message: 'An unknown error occurred',
      }));
      throw new ApiClientError(response.status, error.code, error.message);
    }

    // Handle 204 No Content
    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PUT', path, body);
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>('DELETE', path);
  }
}

export class ApiClientError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'ApiClientError';
  }
}

export const api = new ApiClient(`${API_BASE}/api/v1`);
```

**frontend/src/lib/api/auth.ts:**
```typescript
import { api } from './client';

/**
 * User entity from the API.
 * All IDs use prefixed nanoid format (e.g., usr_V1StGXR8_Z5j)
 */
export interface User {
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  id: string;
  display_name: string;
  email: string | null;
  avatar_url: string | null;
  is_guest: boolean;
  games_played: number;
  total_score: number;
  best_score: number;
}

export const authApi = {
  /** Create a guest session */
  async createGuest(): Promise<User> {
    return api.post<User>('/auth/guest');
  },

  /** Get current authenticated user */
  async getCurrentUser(): Promise<User> {
    return api.get<User>('/auth/me');
  },

  /** Logout and destroy session */
  async logout(): Promise<void> {
    return api.post('/auth/logout');
  },

  /** Get Google OAuth URL */
  getGoogleAuthUrl(redirectTo?: string): string {
    const base = import.meta.env.VITE_API_URL || 'http://localhost:3001';
    const params = redirectTo ? `?redirect_to=${encodeURIComponent(redirectTo)}` : '';
    return `${base}/api/v1/auth/google${params}`;
  },

  /** Get Microsoft OAuth URL */
  getMicrosoftAuthUrl(redirectTo?: string): string {
    const base = import.meta.env.VITE_API_URL || 'http://localhost:3001';
    const params = redirectTo ? `?redirect_to=${encodeURIComponent(redirectTo)}` : '';
    return `${base}/api/v1/auth/microsoft${params}`;
  },
};
```

**frontend/src/lib/api/games.ts:**
```typescript
import { api } from './client';

export interface GameSettings {
  rounds: number;
  time_limit_seconds: number;
  map_id: string;
  movement_allowed: boolean;
  zoom_allowed: boolean;
  rotation_allowed: boolean;
}

export interface CreateGameRequest {
  mode: 'solo' | 'multiplayer';
  settings?: Partial<GameSettings>;
}

/**
 * Response when creating a new game.
 * IDs use prefixed nanoid format.
 */
export interface CreateGameResponse {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  id: string;
  join_code: string | null;
}

export interface Player {
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  user_id: string;
  display_name: string;
  is_host: boolean;
  score: number;
}

export interface GameDetails {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  id: string;
  mode: string;
  status: string;
  created_at: string;
  started_at: string | null;
  ended_at: string | null;
  settings: GameSettings;
  players: Player[];
  current_round: number;
  total_rounds: number;
}

export interface Location {
  lat: number;
  lng: number;
  panorama_id: string | null;
}

export interface RoundInfo {
  round_number: number;
  location: Location;
  started_at: string;
  time_limit_ms: number | null;
}

export interface GuessResult {
  distance_meters: number;
  score: number;
  correct_location: Location;
}

export interface GameSummary {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  id: string;
  mode: string;
  status: string;
  score: number;
  played_at: string;
}

export const gamesApi = {
  /** Create a new game */
  async create(request: CreateGameRequest): Promise<CreateGameResponse> {
    return api.post<CreateGameResponse>('/games', request);
  },

  /** Get game details */
  async get(gameId: string): Promise<GameDetails> {
    return api.get<GameDetails>(`/games/${gameId}`);
  },

  /** Start a game (host only) */
  async start(gameId: string): Promise<RoundInfo> {
    return api.post<RoundInfo>(`/games/${gameId}/start`);
  },

  /** Submit a guess */
  async submitGuess(
    gameId: string,
    roundNumber: number,
    lat: number,
    lng: number,
    timeTakenMs?: number
  ): Promise<GuessResult> {
    return api.post<GuessResult>(`/games/${gameId}/rounds/${roundNumber}/guess`, {
      lat,
      lng,
      time_taken_ms: timeTakenMs,
    });
  },

  /** Get user's game history */
  async getHistory(): Promise<GameSummary[]> {
    return api.get<GameSummary[]>('/games/history');
  },

  /** Join a game by code */
  async joinByCode(code: string): Promise<GameDetails> {
    return api.post<GameDetails>('/games/join', { code });
  },
};
```

### 6.3 Socket.IO Client

**frontend/src/lib/socket/client.ts:**
```typescript
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
```

**frontend/src/lib/socket/game.ts:**
```typescript
import { writable, derived, type Readable } from 'svelte/store';
import { socketClient } from './client';

// Types matching backend protocol
export interface RoundLocation {
  lat: number;
  lng: number;
  panorama_id: string | null;
}

export interface RoundStartPayload {
  round_number: number;
  total_rounds: number;
  location: RoundLocation;
  time_limit_ms: number | null;
  started_at: number;
}

export interface PlayerGuessedPayload {
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  user_id: string;
  display_name: string;
}

export interface RoundResult {
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  user_id: string;
  display_name: string;
  guess_lat: number;
  guess_lng: number;
  distance_meters: number;
  score: number;
  total_score: number;
}

export interface RoundEndPayload {
  round_number: number;
  correct_location: RoundLocation;
  results: RoundResult[];
}

export interface FinalStanding {
  rank: number;
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  user_id: string;
  display_name: string;
  total_score: number;
}

export interface GameEndPayload {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  game_id: string;
  final_standings: FinalStanding[];
}

export interface GameState {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  gameId: string | null;
  status: 'idle' | 'lobby' | 'playing' | 'round_end' | 'finished';
  currentRound: number;
  totalRounds: number;
  location: RoundLocation | null;
  timeLimit: number | null;
  roundStartedAt: number | null;
  hasGuessed: boolean;
  results: RoundResult[];
  finalStandings: FinalStanding[];
  /** Map keyed by user_id (usr_xxxxxxxxxxxx) */
  players: Map<string, { displayName: string; hasGuessed: boolean }>;
}

function createGameStore() {
  const initialState: GameState = {
    gameId: null,
    status: 'idle',
    currentRound: 0,
    totalRounds: 0,
    location: null,
    timeLimit: null,
    roundStartedAt: null,
    hasGuessed: false,
    results: [],
    finalStandings: [],
    players: new Map(),
  };

  const { subscribe, set, update } = writable<GameState>(initialState);

  return {
    subscribe,

    joinGame(gameId: string): void {
      socketClient.emit('game:join', { game_id: gameId });
      update((s) => ({ ...s, gameId, status: 'lobby' }));
    },

    leaveGame(): void {
      update((s) => {
        if (s.gameId) {
          socketClient.emit('game:leave', { game_id: s.gameId });
        }
        return initialState;
      });
    },

    startGame(): void {
      update((s) => {
        if (s.gameId) {
          socketClient.emit('game:start', { game_id: s.gameId });
        }
        return s;
      });
    },

    submitGuess(lat: number, lng: number, timeTakenMs?: number): void {
      update((s) => {
        if (s.gameId && !s.hasGuessed) {
          socketClient.emit('guess:submit', {
            game_id: s.gameId,
            lat,
            lng,
            time_taken_ms: timeTakenMs,
          });
          return { ...s, hasGuessed: true };
        }
        return s;
      });
    },

    // Event handlers
    handleRoundStart(payload: RoundStartPayload): void {
      update((s) => ({
        ...s,
        status: 'playing',
        currentRound: payload.round_number,
        totalRounds: payload.total_rounds,
        location: payload.location,
        timeLimit: payload.time_limit_ms,
        roundStartedAt: payload.started_at,
        hasGuessed: false,
        results: [],
        players: new Map([...s.players].map(([id, p]) => [id, { ...p, hasGuessed: false }])),
      }));
    },

    handlePlayerGuessed(payload: PlayerGuessedPayload): void {
      update((s) => {
        const players = new Map(s.players);
        players.set(payload.user_id, {
          displayName: payload.display_name,
          hasGuessed: true,
        });
        return { ...s, players };
      });
    },

    handleRoundEnd(payload: RoundEndPayload): void {
      update((s) => ({
        ...s,
        status: 'round_end',
        results: payload.results,
        location: payload.correct_location,
      }));
    },

    handleGameEnd(payload: GameEndPayload): void {
      update((s) => ({
        ...s,
        status: 'finished',
        finalStandings: payload.final_standings,
      }));
    },

    handlePlayerJoined(payload: { user_id: string; display_name: string }): void {
      update((s) => {
        const players = new Map(s.players);
        players.set(payload.user_id, {
          displayName: payload.display_name,
          hasGuessed: false,
        });
        return { ...s, players };
      });
    },

    handlePlayerLeft(payload: { user_id: string }): void {
      update((s) => {
        const players = new Map(s.players);
        players.delete(payload.user_id);
        return { ...s, players };
      });
    },

    reset(): void {
      set(initialState);
    },
  };
}

export const gameStore = createGameStore();

// Initialize socket event listeners
export function initGameSocketListeners(): () => void {
  const unsubscribers = [
    socketClient.on<RoundStartPayload>('round:start', (data) => {
      gameStore.handleRoundStart(data);
    }),
    socketClient.on<PlayerGuessedPayload>('player:guessed', (data) => {
      gameStore.handlePlayerGuessed(data);
    }),
    socketClient.on<RoundEndPayload>('round:end', (data) => {
      gameStore.handleRoundEnd(data);
    }),
    socketClient.on<GameEndPayload>('game:end', (data) => {
      gameStore.handleGameEnd(data);
    }),
    socketClient.on<{ user_id: string; display_name: string }>('player:joined', (data) => {
      gameStore.handlePlayerJoined(data);
    }),
    socketClient.on<{ user_id: string }>('player:left', (data) => {
      gameStore.handlePlayerLeft(data);
    }),
  ];

  return () => {
    unsubscribers.forEach((unsub) => unsub());
  };
}
```

### 6.4 Auth Store

**frontend/src/lib/stores/auth.ts:**
```typescript
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
      } catch (error) {
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
```

### 6.5 Layout Components

**frontend/src/routes/+layout.svelte:**
```svelte
<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { authStore, user, isLoading } from '$lib/stores/auth';
  import { socketClient } from '$lib/socket/client';
  import Header from '$lib/components/Header.svelte';

  onMount(async () => {
    await authStore.initialize();
    
    // Connect to realtime if authenticated
    authStore.subscribe(($auth) => {
      if ($auth.user && !socketClient.connected) {
        socketClient.connect();
      }
    });

    return () => {
      socketClient.disconnect();
    };
  });
</script>

<div class="min-h-screen flex flex-col">
  <Header />
  
  <main class="flex-1">
    {#if $isLoading}
      <div class="flex items-center justify-center h-64">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    {:else}
      <slot />
    {/if}
  </main>

  <footer class="bg-gray-100 py-4 text-center text-sm text-gray-600">
    <p>dguesser - A geography guessing game</p>
  </footer>
</div>
```

**frontend/src/lib/components/Header.svelte:**
```svelte
<script lang="ts">
  import { user, isGuest, authStore } from '$lib/stores/auth';
  import { authApi } from '$lib/api/auth';

  async function handleLogout() {
    await authStore.logout();
  }
</script>

<header class="bg-white shadow-sm">
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="flex justify-between items-center h-16">
      <a href="/" class="text-2xl font-bold text-primary-600">
        dguesser
      </a>

      <nav class="flex items-center gap-4">
        {#if $user}
          <a href="/play" class="text-gray-600 hover:text-gray-900">
            Play
          </a>
          <a href="/history" class="text-gray-600 hover:text-gray-900">
            History
          </a>
          
          <div class="flex items-center gap-3 ml-4">
            {#if $user.avatar_url}
              <img 
                src={$user.avatar_url} 
                alt={$user.display_name}
                class="w-8 h-8 rounded-full"
              />
            {:else}
              <div class="w-8 h-8 rounded-full bg-primary-100 flex items-center justify-center">
                <span class="text-primary-600 font-medium">
                  {$user.display_name.charAt(0).toUpperCase()}
                </span>
              </div>
            {/if}
            
            <span class="text-sm font-medium text-gray-700">
              {$user.display_name}
              {#if $isGuest}
                <span class="text-xs text-gray-500">(Guest)</span>
              {/if}
            </span>

            {#if $isGuest}
              <a 
                href={authApi.getGoogleAuthUrl()}
                class="btn-primary text-sm"
              >
                Sign In
              </a>
            {:else}
              <button 
                onclick={handleLogout}
                class="btn-secondary text-sm"
              >
                Logout
              </button>
            {/if}
          </div>
        {:else}
          <a 
            href={authApi.getGoogleAuthUrl()}
            class="btn-primary"
          >
            Sign In with Google
          </a>
        {/if}
      </nav>
    </div>
  </div>
</header>
```

### 6.6 Home Page

**frontend/src/routes/+page.svelte:**
```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { user, authStore } from '$lib/stores/auth';
  import { gamesApi } from '$lib/api/games';
  import { authApi } from '$lib/api/auth';

  let joinCode = '';
  let loading = false;
  let error = '';

  async function startSoloGame() {
    loading = true;
    error = '';

    try {
      // Create guest if not logged in
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'solo' });
      goto(`/game/${game.id}`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to start game';
    } finally {
      loading = false;
    }
  }

  async function createMultiplayerGame() {
    loading = true;
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.create({ mode: 'multiplayer' });
      goto(`/game/${game.id}/lobby`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create game';
    } finally {
      loading = false;
    }
  }

  async function joinGame() {
    if (!joinCode.trim()) return;

    loading = true;
    error = '';

    try {
      if (!$user) {
        await authStore.createGuest();
      }

      const game = await gamesApi.joinByCode(joinCode.trim().toUpperCase());
      goto(`/game/${game.id}/lobby`);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to join game';
    } finally {
      loading = false;
    }
  }
</script>

<div class="max-w-4xl mx-auto px-4 py-12">
  <div class="text-center mb-12">
    <h1 class="text-5xl font-bold text-gray-900 mb-4">
      Welcome to dguesser
    </h1>
    <p class="text-xl text-gray-600">
      Test your geography knowledge by guessing locations around the world
    </p>
  </div>

  {#if error}
    <div class="mb-6 p-4 bg-red-50 border border-red-200 rounded-lg text-red-700">
      {error}
    </div>
  {/if}

  <div class="grid md:grid-cols-2 gap-6">
    <!-- Solo Play -->
    <div class="card">
      <h2 class="text-2xl font-semibold mb-4">Solo Play</h2>
      <p class="text-gray-600 mb-6">
        Play on your own and try to beat your high score. No account required.
      </p>
      <button
        onclick={startSoloGame}
        disabled={loading}
        class="btn-primary w-full"
      >
        {loading ? 'Starting...' : 'Start Solo Game'}
      </button>
    </div>

    <!-- Multiplayer -->
    <div class="card">
      <h2 class="text-2xl font-semibold mb-4">Multiplayer</h2>
      <p class="text-gray-600 mb-4">
        Create a room or join friends with a code.
      </p>
      
      <button
        onclick={createMultiplayerGame}
        disabled={loading}
        class="btn-accent w-full mb-4"
      >
        Create Room
      </button>

      <div class="flex gap-2">
        <input
          type="text"
          bind:value={joinCode}
          placeholder="Enter join code"
          maxlength="6"
          class="input uppercase"
        />
        <button
          onclick={joinGame}
          disabled={loading || !joinCode.trim()}
          class="btn-secondary"
        >
          Join
        </button>
      </div>
    </div>
  </div>

  <!-- Sign in prompt for guests -->
  {#if $user?.is_guest}
    <div class="mt-8 p-6 bg-primary-50 rounded-xl text-center">
      <h3 class="text-lg font-semibold text-primary-900 mb-2">
        Want to save your progress?
      </h3>
      <p class="text-primary-700 mb-4">
        Sign in to track your scores and compete on leaderboards.
      </p>
      <div class="flex justify-center gap-4">
        <a href={authApi.getGoogleAuthUrl()} class="btn-primary">
          Sign in with Google
        </a>
        <a href={authApi.getMicrosoftAuthUrl()} class="btn-secondary">
          Sign in with Microsoft
        </a>
      </div>
    </div>
  {/if}

  <!-- Stats for logged in users -->
  {#if $user && !$user.is_guest}
    <div class="mt-8 grid grid-cols-3 gap-4">
      <div class="card text-center">
        <div class="text-3xl font-bold text-primary-600">
          {$user.games_played}
        </div>
        <div class="text-gray-600">Games Played</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-accent-600">
          {$user.total_score.toLocaleString()}
        </div>
        <div class="text-gray-600">Total Score</div>
      </div>
      <div class="card text-center">
        <div class="text-3xl font-bold text-yellow-600">
          {$user.best_score.toLocaleString()}
        </div>
        <div class="text-gray-600">Best Score</div>
      </div>
    </div>
  {/if}
</div>
```

### 6.7 Auth Callback Page

**frontend/src/routes/auth/success/+page.svelte:**
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { authStore } from '$lib/stores/auth';

  onMount(async () => {
    // Refresh user data after OAuth callback
    await authStore.initialize();
    
    // Redirect to home or stored redirect
    const redirectTo = sessionStorage.getItem('auth_redirect') || '/';
    sessionStorage.removeItem('auth_redirect');
    goto(redirectTo);
  });
</script>

<div class="flex items-center justify-center min-h-[60vh]">
  <div class="text-center">
    <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto mb-4"></div>
    <p class="text-gray-600">Completing sign in...</p>
  </div>
</div>
```

## Project Structure Summary

```
frontend/
├── src/
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts       # Base API client
│   │   │   ├── auth.ts         # Auth API
│   │   │   └── games.ts        # Games API
│   │   ├── socket/
│   │   │   ├── client.ts       # Socket.IO client
│   │   │   └── game.ts         # Game socket store
│   │   ├── stores/
│   │   │   └── auth.ts         # Auth store
│   │   └── components/
│   │       └── Header.svelte   # Header component
│   ├── routes/
│   │   ├── +layout.svelte      # Root layout
│   │   ├── +page.svelte        # Home page
│   │   └── auth/
│   │       └── success/
│   │           └── +page.svelte
│   └── app.css                 # Global styles
├── tailwind.config.js
├── svelte.config.js
└── package.json
```

## Acceptance Criteria

- [ ] SvelteKit dev server starts
- [ ] Tailwind styles apply correctly
- [ ] API client makes authenticated requests
- [ ] Socket.IO connects and authenticates
- [ ] Auth store initializes on page load
- [ ] Guest creation works
- [ ] OAuth redirects work
- [ ] Header shows user info
- [ ] Home page renders correctly
- [ ] **All ID types documented with prefixed format**

## Technical Notes

### ID Format Reference (Frontend)

All entity IDs from the API use prefixed nanoid format. In TypeScript, these are `string` types:

| Entity | Format | Example |
|--------|--------|---------|
| User ID | `usr_xxxxxxxxxxxx` | `usr_V1StGXR8_Z5j` |
| Game ID | `gam_xxxxxxxxxxxx` | `gam_FybH2oF9Xaw8` |
| Round ID | `rnd_xxxxxxxxxxxx` | `rnd_Q3kT7bN2mPxW` |

**Best Practices:**
- Always use `string` type for IDs (not `number` or custom types)
- Include JSDoc comments documenting the expected format
- IDs are URL-safe and can be used directly in routes

## Next Phase

Once frontend foundation is complete, proceed to [Phase 7: Game Experience](./07-game-experience.md).
