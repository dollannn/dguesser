/**
 * MAP-007: Shared types for coordinates and location data
 * 
 * These types provide consistency across all map-related components
 * and prevent the use of inconsistent coordinate representations.
 */

/**
 * Basic latitude/longitude coordinate pair
 */
export interface Coordinates {
  lat: number;
  lng: number;
}

/**
 * Coordinates with optional identifier
 */
export interface CoordinatesWithId extends Coordinates {
  /** Location ID from the database */
  id?: string;
  /** Street View panorama ID */
  panoramaId?: string;
}

/**
 * Full location data as used during gameplay
 */
export interface GameLocationData extends CoordinatesWithId {
  /** ISO 3166-1 alpha-2 country code */
  countryCode?: string;
  /** Default heading/direction for panorama (degrees, 0-360) */
  heading?: number;
}

/**
 * A player's guess
 */
export interface GuessData extends Coordinates {
  /** Distance from correct location in meters */
  distanceMeters?: number;
  /** Score earned for this guess */
  score?: number;
  /** Time taken to submit guess in milliseconds */
  timeTakenMs?: number;
}

/**
 * Round result with correct location and player's guess
 */
export interface RoundResult {
  /** Round number (1-indexed) */
  roundNumber: number;
  /** The correct location */
  correctLocation: Coordinates;
  /** Player's guess (if submitted) */
  guess?: GuessData;
}
