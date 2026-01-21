import { writable, get } from 'svelte/store';
import { socketClient, toastStore, type GamePhase } from './client';
import type { GameSettings } from '$lib/api/games';
import { authStore } from '$lib/stores/auth';

// Types matching backend protocol
export interface RoundLocation {
  lat: number;
  lng: number;
  panorama_id: string | null;
  /** Location ID for reporting (loc_xxxxxxxxxxxx) */
  location_id?: string | null;
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

/** Player info from game state (includes connection status) */
export interface PlayerInfo {
  id: string;
  display_name: string;
  avatar_url: string | null;
  score: number;
  has_guessed: boolean;
  connected?: boolean;
  disconnected_at?: number | null;
}

/** Full game state payload (sent on join/reconnect) */
export interface GameStatePayload {
  game_id: string;
  status: string;
  current_round: number;
  total_rounds: number;
  settings: GameSettings;
  host_id: string;
  players: PlayerInfo[];
  location: RoundLocation | null;
  time_remaining_ms: number | null;
}

/** Settings updated payload */
export interface SettingsUpdatedPayload {
  game_id: string;
  settings: GameSettings;
}

/** Player disconnected payload */
export interface PlayerDisconnectedPayload {
  user_id: string;
  display_name: string;
  /** Grace period in ms, or null/undefined if player won't be kicked (mid-game disconnects) */
  grace_period_ms?: number | null;
}

/** Player reconnected payload */
export interface PlayerReconnectedPayload {
  user_id: string;
  display_name: string;
}

/** Player timed out payload */
export interface PlayerTimedOutPayload {
  user_id: string;
  display_name: string;
}

/** Game abandoned payload (all players disconnected for too long) */
export interface GameAbandonedPayload {
  game_id: string;
  reason: string;
}

/** Live scores update payload */
export interface ScoresUpdatePayload {
  round_number: number;
  total_rounds: number;
  scores: PlayerScoreInfo[];
}

/** Player score info for live scoreboard */
export interface PlayerScoreInfo {
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  total_score: number;
  round_score: number;
  has_guessed: boolean;
  rank: number;
  connected: boolean;
}

/** Extended player state in store */
export interface PlayerState {
  displayName: string;
  avatarUrl: string | null;
  score: number;
  hasGuessed: boolean;
  connected: boolean;
  disconnectedAt: number | null;
}

export interface GameState {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  gameId: string | null;
  status: 'idle' | 'lobby' | 'playing' | 'round_end' | 'finished';
  currentRound: number;
  totalRounds: number;
  /** Game settings */
  settings: GameSettings | null;
  /** Host user ID (usr_xxxxxxxxxxxx) */
  hostId: string | null;
  location: RoundLocation | null;
  timeLimit: number | null;
  roundStartedAt: number | null;
  timeRemainingMs: number | null;
  hasGuessed: boolean;
  results: RoundResult[];
  /** All rounds' results for end-of-game statistics (solo mode) */
  roundHistory: RoundResult[][];
  finalStandings: FinalStanding[];
  /** Map keyed by user_id (usr_xxxxxxxxxxxx) */
  players: Map<string, PlayerState>;
  /** Live scoreboard data */
  liveScores: PlayerScoreInfo[];
}

function createGameStore() {
  const initialState: GameState = {
    gameId: null,
    status: 'idle',
    currentRound: 0,
    totalRounds: 0,
    settings: null,
    hostId: null,
    location: null,
    timeLimit: null,
    roundStartedAt: null,
    timeRemainingMs: null,
    hasGuessed: false,
    results: [],
    roundHistory: [],
    finalStandings: [],
    players: new Map(),
    liveScores: [],
  };

  const { subscribe, set, update } = writable<GameState>(initialState);

  return {
    subscribe,

    async joinGame(gameId: string): Promise<void> {
      // Wait for socket authentication before joining
      await socketClient.waitForAuth();
      socketClient.emit('game:join', { game_id: gameId });
      // Set as lobby phase - will be updated to 'active' when game starts
      socketClient.setActiveGame(gameId, 'lobby');
      update((s) => ({ ...s, gameId, status: 'lobby' }));
    },

    leaveGame(): void {
      const currentState = get({ subscribe });
      if (currentState.gameId) {
        socketClient.emit('game:leave', { game_id: currentState.gameId });
      }
      socketClient.setActiveGame(null, null);
      set(initialState);
    },

    startGame(): void {
      const currentState = get({ subscribe });
      if (currentState.gameId) {
        socketClient.emit('game:start', { game_id: currentState.gameId });
      }
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

    /** Handle full game state sync (on join or reconnect) */
    handleGameState(payload: GameStatePayload): void {
      const players = new Map<string, PlayerState>();
      for (const p of payload.players) {
        players.set(p.id, {
          displayName: p.display_name,
          avatarUrl: p.avatar_url,
          score: p.score,
          hasGuessed: p.has_guessed,
          connected: p.connected ?? true,
          disconnectedAt: p.disconnected_at ?? null,
        });
      }

      // Map server status to client status and determine phase for reconnection logic
      let status: GameState['status'];
      let socketPhase: GamePhase;
      switch (payload.status) {
        case 'lobby':
          status = 'lobby';
          socketPhase = 'lobby';
          break;
        case 'active':
          status = payload.location ? 'playing' : 'lobby';
          socketPhase = 'active'; // Active games allow auto-rejoin
          break;
        case 'finished':
          status = 'finished';
          socketPhase = null; // No need to rejoin finished games
          break;
        default:
          status = 'lobby';
          socketPhase = 'lobby';
      }

      // Update socket client's phase for reconnection logic
      socketClient.setActiveGamePhase(socketPhase);

      // Calculate round started time from time remaining
      let roundStartedAt: number | null = null;
      if (payload.time_remaining_ms !== null && payload.location) {
        // Estimate when round started based on remaining time
        roundStartedAt = Date.now() - (payload.time_remaining_ms ?? 0);
      }

      // Generate liveScores from players for scoreboard display
      // This ensures the scoreboard shows immediately after join/reconnect
      const liveScores: PlayerScoreInfo[] = payload.players
        .map((p) => ({
          user_id: p.id,
          display_name: p.display_name,
          avatar_url: p.avatar_url,
          total_score: p.score,
          round_score: 0, // Not available in game state, will be updated by scores:update
          has_guessed: p.has_guessed,
          rank: 0, // Will be assigned after sorting
          connected: p.connected ?? true,
        }))
        .sort((a, b) => b.total_score - a.total_score)
        .map((p, i) => ({ ...p, rank: i + 1 }));

      update((s) => ({
        ...s,
        gameId: payload.game_id,
        status,
        currentRound: payload.current_round,
        totalRounds: payload.total_rounds,
        settings: payload.settings,
        hostId: payload.host_id,
        location: payload.location,
        timeRemainingMs: payload.time_remaining_ms,
        roundStartedAt,
        // Preserve hasGuessed if we're the one who already guessed
        hasGuessed: payload.players.some((p) => p.has_guessed && p.id === getCurrentUserId()),
        players,
        liveScores,
      }));
    },

    handleRoundStart(payload: RoundStartPayload): void {
      // Game is now active - update phase for reconnection logic
      socketClient.setActiveGamePhase('active');

      update((s) => ({
        ...s,
        status: 'playing',
        currentRound: payload.round_number,
        totalRounds: payload.total_rounds,
        location: payload.location,
        timeLimit: payload.time_limit_ms,
        roundStartedAt: payload.started_at,
        timeRemainingMs: payload.time_limit_ms,
        hasGuessed: false,
        results: [],
        players: new Map(
          [...s.players].map(([id, p]) => [id, { ...p, hasGuessed: false }])
        ),
      }));
    },

    handlePlayerGuessed(payload: PlayerGuessedPayload): void {
      update((s) => {
        const players = new Map(s.players);
        const existing = players.get(payload.user_id);
        if (existing) {
          players.set(payload.user_id, { ...existing, hasGuessed: true });
        } else {
          players.set(payload.user_id, {
            displayName: payload.display_name,
            avatarUrl: null,
            score: 0,
            hasGuessed: true,
            connected: true,
            disconnectedAt: null,
          });
        }
        return { ...s, players };
      });
    },

    handleRoundEnd(payload: RoundEndPayload): void {
      update((s) => {
        // Update player scores from results
        const players = new Map(s.players);
        for (const result of payload.results) {
          const existing = players.get(result.user_id);
          if (existing) {
            players.set(result.user_id, {
              ...existing,
              score: result.total_score,
            });
          }
        }

        return {
          ...s,
          status: 'round_end',
          results: payload.results,
          // Accumulate round history for end-of-game statistics
          roundHistory: [...s.roundHistory, payload.results],
          location: payload.correct_location,
          players,
        };
      });
    },

    handleGameEnd(payload: GameEndPayload): void {
      socketClient.setActiveGame(null, null);
      update((s) => ({
        ...s,
        status: 'finished',
        finalStandings: payload.final_standings,
      }));
    },

    /** Handle game abandoned (all players disconnected for too long) */
    handleGameAbandoned(payload: GameAbandonedPayload): void {
      socketClient.setActiveGame(null, null);
      update((s) => ({
        ...s,
        status: 'finished',
        finalStandings: [],
      }));
      toastStore.add('error', `Game abandoned: ${payload.reason}`);
    },

    handlePlayerJoined(payload: { player: PlayerInfo }): void {
      update((s) => {
        const players = new Map(s.players);
        players.set(payload.player.id, {
          displayName: payload.player.display_name,
          avatarUrl: payload.player.avatar_url,
          score: payload.player.score,
          hasGuessed: payload.player.has_guessed,
          connected: true,
          disconnectedAt: null,
        });
        return { ...s, players };
      });
    },

    handlePlayerLeft(payload: { user_id: string }): void {
      // Check if this is the current user being removed
      const currentUserId = getCurrentUserId();
      if (payload.user_id === currentUserId) {
        // We were removed from the game - clear state and prevent auto-rejoin
        socketClient.setActiveGame(null, null);
        set(initialState);
        toastStore.add('info', 'You have left the game.');
        return;
      }

      update((s) => {
        const players = new Map(s.players);
        players.delete(payload.user_id);
        return { ...s, players };
      });
    },

    /** Handle player disconnection (grace period started) */
    handlePlayerDisconnected(payload: PlayerDisconnectedPayload): void {
      update((s) => {
        const players = new Map(s.players);
        const existing = players.get(payload.user_id);
        if (existing) {
          players.set(payload.user_id, {
            ...existing,
            connected: false,
            disconnectedAt: Date.now(),
          });
        }
        return { ...s, players };
      });
      toastStore.add('info', `${payload.display_name} disconnected`);
    },

    /** Handle player reconnection (within grace period) */
    handlePlayerReconnected(payload: PlayerReconnectedPayload): void {
      update((s) => {
        const players = new Map(s.players);
        const existing = players.get(payload.user_id);
        if (existing) {
          players.set(payload.user_id, {
            ...existing,
            connected: true,
            disconnectedAt: null,
          });
        }
        return { ...s, players };
      });
      toastStore.add('success', `${payload.display_name} reconnected`);
    },

    /** Handle player timeout (grace period expired) */
    handlePlayerTimedOut(payload: PlayerTimedOutPayload): void {
      update((s) => {
        const players = new Map(s.players);
        players.delete(payload.user_id);
        return { ...s, players };
      });
      toastStore.add('warning', `${payload.display_name} timed out`);
    },

    /** Handle live scores update */
    handleScoresUpdate(payload: ScoresUpdatePayload): void {
      update((s) => ({
        ...s,
        liveScores: payload.scores,
        currentRound: payload.round_number,
        totalRounds: payload.total_rounds,
      }));
    },

    /** Handle settings updated (in lobby) */
    handleSettingsUpdated(payload: SettingsUpdatedPayload): void {
      update((s) => ({
        ...s,
        settings: payload.settings,
        totalRounds: payload.settings.rounds,
      }));
    },

    /** Set round info (for restoring state) */
    setRoundInfo(currentRound: number, totalRounds: number): void {
      update((s) => ({
        ...s,
        currentRound,
        totalRounds,
      }));
    },

    reset(): void {
      socketClient.setActiveGame(null, null);
      set(initialState);
    },
  };
}

/** Get current user ID from auth store */
function getCurrentUserId(): string | null {
  const authState = get(authStore);
  return authState.user?.id ?? null;
}

export const gameStore = createGameStore();

// Initialize socket event listeners
export function initGameSocketListeners(): () => void {
  const unsubscribers = [
    // Full game state sync (on join/reconnect)
    socketClient.on<GameStatePayload>('game:state', (data) => {
      gameStore.handleGameState(data);
    }),
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
    socketClient.on<GameAbandonedPayload>('game:abandoned', (data) => {
      gameStore.handleGameAbandoned(data);
    }),
    socketClient.on<{ player: PlayerInfo }>('player:joined', (data) => {
      gameStore.handlePlayerJoined(data);
    }),
    socketClient.on<{ user_id: string }>('player:left', (data) => {
      gameStore.handlePlayerLeft(data);
    }),
    // Reconnection events
    socketClient.on<PlayerDisconnectedPayload>('player:disconnected', (data) => {
      gameStore.handlePlayerDisconnected(data);
    }),
    socketClient.on<PlayerReconnectedPayload>('player:reconnected', (data) => {
      gameStore.handlePlayerReconnected(data);
    }),
    socketClient.on<PlayerTimedOutPayload>('player:timeout', (data) => {
      gameStore.handlePlayerTimedOut(data);
    }),
    // Live scores update
    socketClient.on<ScoresUpdatePayload>('scores:update', (data) => {
      gameStore.handleScoresUpdate(data);
    }),
    // Settings updated (in lobby)
    socketClient.on<SettingsUpdatedPayload>('game:settings_updated', (data) => {
      gameStore.handleSettingsUpdated(data);
    }),
    // Error handling
    socketClient.on<{ code: string; message: string }>('error', (data) => {
      console.error('[Socket Error]', data.code, data.message);
      toastStore.add('error', data.message);
    }),
  ];

  return () => {
    unsubscribers.forEach((unsub) => unsub());
  };
}
