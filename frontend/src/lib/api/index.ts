// API client module
export { api, ApiClientError } from './client';
export { authApi, type User } from './auth';
export {
  gamesApi,
  type GameSettings,
  type CreateGameRequest,
  type CreateGameResponse,
  type Player,
  type GameDetails,
  type Location,
  type RoundInfo,
  type GuessResult,
  type GameSummary,
} from './games';
export {
  leaderboardApi,
  type LeaderboardType,
  type TimePeriod,
  type LeaderboardEntry,
  type LeaderboardResponse,
  type LeaderboardQuery,
} from './leaderboard';
