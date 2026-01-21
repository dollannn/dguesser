/**
 * Map Pin SVG Generator
 *
 * Creates consistent teardrop-shaped map pins for use with Leaflet markers.
 * Supports customizable colors, animations (pulse, bounce), and opacity.
 */

import type L from 'leaflet';

export interface MapPinOptions {
  /** Fill color for the pin (default: #22c55e - green) */
  color?: string;
  /** Size in pixels (default: 32) */
  size?: number;
  /** Enable pulse animation - for correct location markers */
  pulse?: boolean;
  /** Enable bounce animation - for placement feedback */
  bounce?: boolean;
  /** Opacity 0-1 (default: 1) - for hover preview */
  opacity?: number;
  /** Optional CSS class to add to the container */
  className?: string;
}

const DEFAULT_OPTIONS: Required<Omit<MapPinOptions, 'className'>> = {
  color: '#22c55e',
  size: 32,
  pulse: false,
  bounce: false,
  opacity: 1,
};

/**
 * Creates the SVG markup for a clean map pin.
 *
 * Simple, modern design with:
 * - Clean teardrop silhouette
 * - Solid color fill
 * - Small inner dot
 * - Subtle drop shadow
 */
export function createMapPinSvg(options: MapPinOptions = {}): string {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const { color, size } = opts;

  // Pin is 24x32 aspect ratio
  const width = size;
  const height = Math.round(size * 1.33);

  const svgContent = `
    <svg 
      xmlns="http://www.w3.org/2000/svg" 
      width="${width}" 
      height="${height}" 
      viewBox="0 0 24 32"
      style="filter: drop-shadow(0 2px 3px rgba(0,0,0,0.35));"
    >
      <!-- Pin shape with transparent center hole (using evenodd fill rule) -->
      <path 
        fill-rule="evenodd"
        d="M12 0C5.373 0 0 5.373 0 12c0 4.418 4 10.667 12 20 8-9.333 12-15.582 12-20C24 5.373 18.627 0 12 0zm0 7a4 4 0 100 8 4 4 0 000-8z"
        fill="${color}"
      />
    </svg>
  `.trim();

  return svgContent;
}

/**
 * Creates a Leaflet DivIcon with the map pin SVG.
 *
 * @param L - Leaflet library reference
 * @param options - Pin customization options
 * @returns Leaflet DivIcon configured with the pin SVG
 */
export function createMapPinIcon(
  L: typeof import('leaflet'),
  options: MapPinOptions = {}
): L.DivIcon {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const { size, pulse, bounce, opacity, className } = opts;

  const height = Math.round(size * 1.33);
  const svg = createMapPinSvg(opts);

  // Pulse ring for correct location marker
  const pulseRing = pulse
    ? `<div class="map-pin-pulse" style="
        position: absolute;
        top: 0;
        left: 50%;
        transform: translateX(-50%);
        width: ${size}px;
        height: ${size}px;
        border-radius: 50%;
        background-color: ${opts.color};
        opacity: 0.4;
        animation: map-pin-pulse 2s ease-out infinite;
        pointer-events: none;
      "></div>`
    : '';

  const styles = [
    'position: relative;',
    opacity < 1 ? `opacity: ${opacity};` : '',
    bounce ? 'animation: map-pin-bounce 0.3s ease-out;' : '',
  ]
    .filter(Boolean)
    .join(' ');

  const html = `
    <div class="map-pin-container ${className || ''}" style="${styles}">
      ${pulseRing}
      ${svg}
    </div>
  `.trim();

  return L.divIcon({
    className: 'map-pin-icon',
    html,
    iconSize: [size, height],
    iconAnchor: [size / 2, height], // Anchor at bottom center (pin tip)
  });
}

/**
 * CSS for map pin animations.
 * Include this in a global stylesheet or component style block.
 */
export const MAP_PIN_STYLES = `
  .map-pin-icon {
    background: transparent !important;
    border: none !important;
  }

  @keyframes map-pin-pulse {
    0% {
      transform: translateX(-50%) scale(0.8);
      opacity: 0.6;
    }
    100% {
      transform: translateX(-50%) scale(2);
      opacity: 0;
    }
  }

  @keyframes map-pin-bounce {
    0% {
      transform: translateY(-8px);
      opacity: 0;
    }
    60% {
      transform: translateY(2px);
    }
    100% {
      transform: translateY(0);
      opacity: 1;
    }
  }
`;
