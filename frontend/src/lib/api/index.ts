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
export {
  usersApi,
  sessionsApi,
  type UserProfile,
  type UpdateProfileRequest,
  type DeleteAccountResponse,
  type SessionInfo,
  type SessionsListResponse,
  type RevokeSessionResponse,
} from './users';
export {
  mapsApi,
  locationsApi,
  type MapVisibility,
  type MapSummary,
  type MapDetails,
  type CreateMapRequest,
  type CreateMapResponse,
  type UpdateMapRequest,
  type MapLocationItem,
  type MapLocationsResponse,
  type AddLocationsRequest,
  type AddLocationsResponse,
  type AddLocationsFromUrlsRequest,
  type AddLocationsFromUrlsResponse,
  type UrlParseResult,
  type LocationSearchFilters,
  type LocationSearchItem,
  type SearchLocationsResponse,
  type CountryInfo,
  type CountriesResponse,
  type SubdivisionInfo,
  type SubdivisionsResponse,
} from './maps';
