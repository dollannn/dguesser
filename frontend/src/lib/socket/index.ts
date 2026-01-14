// Socket.IO client module
export { socketClient, type ConnectionStatus } from './client';
export {
  gameStore,
  initGameSocketListeners,
  type RoundLocation,
  type RoundStartPayload,
  type PlayerGuessedPayload,
  type RoundResult,
  type RoundEndPayload,
  type FinalStanding,
  type GameEndPayload,
  type GameState,
} from './game';
