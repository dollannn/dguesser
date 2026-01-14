import { writable } from 'svelte/store';
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
