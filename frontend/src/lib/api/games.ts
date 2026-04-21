import { api } from './client';

export type GameMode = 'solo' | 'multiplayer' | 'challenge';
export type GameStatus = 'lobby' | 'active' | 'finished' | 'abandoned';

export interface GameSettings {
  rounds: number;
  time_limit_seconds: number;
  map_id: string;
  movement_allowed: boolean;
  zoom_allowed: boolean;
  rotation_allowed: boolean;
}

export interface CreateGameRequest {
  mode: GameMode;
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
  /** Avatar URL from OAuth provider */
  avatar_url: string | null;
  is_host: boolean;
  /** Whether this player is a guest (not signed in with OAuth) */
  is_guest: boolean;
  score: number;
}

export interface GameDetails {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  id: string;
  mode: GameMode;
  status: GameStatus;
  /** Join code for multiplayer games (6 alphanumeric chars) */
  join_code: string | null;
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
  heading?: number | null;
  /** Location ID for reporting (loc_xxxxxxxxxxxx) */
  location_id?: string | null;
}

export interface RoundInfo {
  round_number: number;
  location: Location;
  started_at: string;
  time_limit_ms: number | null;
}

export interface CurrentRoundInfo {
  round_number: number;
  total_rounds: number;
  location: Location;
  started_at: string;
  time_remaining_ms: number | null;
  has_guessed: boolean;
  user_guess: UserGuessInfo | null;
}

export interface UserGuessInfo {
  guess_lat: number;
  guess_lng: number;
  distance_meters: number;
  score: number;
}

export interface GuessResult {
  distance_meters: number;
  score: number;
  total_score: number;
  correct_location: Location;
}

export interface RoundResultInfo {
  user_id: string;
  display_name: string;
  guess_lat: number;
  guess_lng: number;
  distance_meters: number;
  score: number;
  total_score: number;
}

export interface FinalStandingInfo {
  rank: number;
  user_id: string;
  display_name: string;
  total_score: number;
}

export interface CompletedRoundInfo {
  round_number: number;
  correct_location: Location;
  results: RoundResultInfo[];
}

export interface GameResultsResponse {
  game_id: string;
  final_standings: FinalStandingInfo[];
  rounds: CompletedRoundInfo[];
}

export interface GameSummary {
  /** Game ID (prefixed nanoid: gam_xxxxxxxxxxxx) */
  id: string;
  mode: GameMode;
  status: GameStatus;
  score: number;
  played_at: string;
}

export interface UpdateSettingsRequest {
  rounds?: number;
  time_limit_seconds?: number;
  map_id?: string;
  movement_allowed?: boolean;
  zoom_allowed?: boolean;
  rotation_allowed?: boolean;
}

export interface UpdateSettingsResponse {
  settings: GameSettings;
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

  /** Get current round info (for resuming in-progress games) */
  async getCurrentRound(gameId: string): Promise<CurrentRoundInfo> {
    return api.get<CurrentRoundInfo>(`/games/${gameId}/rounds/current`);
  },

  /** Advance to the next round (solo games only) */
  async nextRound(gameId: string): Promise<RoundInfo> {
    return api.post<RoundInfo>(`/games/${gameId}/rounds/next`);
  },

  /** Record a timed-out solo round with no guess */
  async timeoutRound(gameId: string, roundNumber: number): Promise<GuessResult> {
    return api.post<GuessResult>(`/games/${gameId}/rounds/${roundNumber}/timeout`);
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

  /** Get persisted round-by-round results for a finished game */
  async getResults(gameId: string): Promise<GameResultsResponse> {
    return api.get<GameResultsResponse>(`/games/${gameId}/results`);
  },

  /** Join a game by code */
  async joinByCode(code: string): Promise<GameDetails> {
    return api.post<GameDetails>('/games/join', { code });
  },

  /** Update game settings (host only, lobby only) */
  async updateSettings(gameId: string, settings: UpdateSettingsRequest): Promise<UpdateSettingsResponse> {
    return api.patch<UpdateSettingsResponse>(`/games/${gameId}/settings`, settings);
  },
};
