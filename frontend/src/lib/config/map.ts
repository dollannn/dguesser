/**
 * MAP-006: Centralized map configuration
 * 
 * Contains tile provider URL, default settings, and constants
 * for Leaflet maps throughout the application.
 */

/**
 * Map tile provider configuration
 */
export const MAP_TILES = {
  /** 
   * Tile URL template for the map provider.
   * Can be overridden via VITE_MAP_TILES_URL environment variable.
   */
  url: import.meta.env.VITE_MAP_TILES_URL || 
    'https://{s}.basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png',
  
  /** Attribution text for the tile provider */
  attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>',
  
  /** Maximum zoom level supported */
  maxZoom: 19,
};

/**
 * Default map view settings
 */
export const MAP_DEFAULTS = {
  /** Default center coordinates [lat, lng] - Central Europe */
  center: [48, 10] as [number, number],
  
  /** Default zoom level for initial view */
  zoom: 2,
  
  /** Zoom level for result/answer views */
  resultZoom: 4,
  
  /** Padding for fitBounds operations [vertical, horizontal] */
  padding: [50, 50] as [number, number],
};

/**
 * Marker appearance settings
 */
export const MARKER_CONFIG = {
  /** Base size of marker icon in pixels */
  size: 32,

  /** Marker colors by type */
  colors: {
    /** Color for guess markers (Tailwind blue-500) */
    guess: '#3b82f6',
    /** Color for correct location markers (Tailwind green-500) */
    correct: '#22c55e',
    /** Colors for multiplayer guess markers - cycles through array */
    players: ['#3b82f6', '#a855f7', '#f59e0b', '#ec4899', '#06b6d4', '#14b8a6'],
  },

  /** Hover preview opacity */
  hoverOpacity: 0.5,
};
