/**
 * Map Pin SVG Generator
 *
 * Creates consistent map markers for use with Leaflet:
 * - Teardrop pins for player guesses (with optional glow for current user)
 * - Target/crosshair icon for correct location (visually distinct)
 * - Numbered circle markers for game summary
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
  /** Enable glow effect - for current user's marker on results */
  glow?: boolean;
  /** Optional CSS class to add to the container */
  className?: string;
}

const DEFAULT_OPTIONS: Required<Omit<MapPinOptions, 'className'>> = {
  color: '#22c55e',
  size: 32,
  pulse: false,
  bounce: false,
  opacity: 1,
  glow: false,
};

/**
 * Creates the SVG markup for a clean map pin (teardrop shape).
 * Used for player guess markers.
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
 * Creates the SVG markup for a target/crosshair icon.
 * Used for correct location markers — visually distinct from guess pins.
 */
export function createTargetSvg(options: { color?: string; size?: number } = {}): string {
  const { color = '#22c55e', size = 44 } = options;

  return `
    <svg 
      xmlns="http://www.w3.org/2000/svg" 
      width="${size}" 
      height="${size}" 
      viewBox="0 0 40 40"
      style="filter: drop-shadow(0 2px 4px rgba(0,0,0,0.3));"
    >
      <!-- Outer ring -->
      <circle cx="20" cy="20" r="16" fill="none" stroke="${color}" stroke-width="3" opacity="0.4" />
      <!-- Inner ring -->
      <circle cx="20" cy="20" r="10" fill="none" stroke="${color}" stroke-width="2.5" opacity="0.7" />
      <!-- Center dot -->
      <circle cx="20" cy="20" r="4.5" fill="${color}" />
      <!-- Crosshair lines -->
      <line x1="20" y1="0" x2="20" y2="8" stroke="${color}" stroke-width="2.5" stroke-linecap="round" opacity="0.6" />
      <line x1="20" y1="32" x2="20" y2="40" stroke="${color}" stroke-width="2.5" stroke-linecap="round" opacity="0.6" />
      <line x1="0" y1="20" x2="8" y2="20" stroke="${color}" stroke-width="2.5" stroke-linecap="round" opacity="0.6" />
      <line x1="32" y1="20" x2="40" y2="20" stroke="${color}" stroke-width="2.5" stroke-linecap="round" opacity="0.6" />
    </svg>
  `.trim();
}

/**
 * Creates the SVG markup for a numbered circle marker.
 * Used in game summary map to label rounds (1, 2, 3...).
 */
export function createNumberedCircleSvg(
  options: { number: number; color?: string; size?: number; textColor?: string }
): string {
  const { number, color = '#22c55e', size = 28, textColor = '#fff' } = options;

  return `
    <svg 
      xmlns="http://www.w3.org/2000/svg" 
      width="${size}" 
      height="${size}" 
      viewBox="0 0 28 28"
      style="filter: drop-shadow(0 2px 3px rgba(0,0,0,0.3));"
    >
      <circle cx="14" cy="14" r="13" fill="${color}" stroke="#fff" stroke-width="2" />
      <text x="14" y="14" text-anchor="middle" dominant-baseline="central" 
            font-family="system-ui, sans-serif" font-size="13" font-weight="700" fill="${textColor}">
        ${number}
      </text>
    </svg>
  `.trim();
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
  const { size, pulse, bounce, opacity, glow, className } = opts;

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

  // Glow effect for current user's marker
  const glowRing = glow
    ? `<div class="map-pin-glow" style="
        position: absolute;
        top: -4px;
        left: 50%;
        transform: translateX(-50%);
        width: ${size + 8}px;
        height: ${size + 8}px;
        border-radius: 50%;
        background: radial-gradient(circle, ${opts.color}40 0%, ${opts.color}00 70%);
        animation: map-pin-glow 2s ease-in-out infinite;
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
      ${glowRing}
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
 * Creates a Leaflet DivIcon with the target/crosshair SVG.
 * Anchored at center (not bottom) since it's a symmetric icon.
 */
export function createTargetIcon(
  L: typeof import('leaflet'),
  options: { color?: string; size?: number; pulse?: boolean } = {}
): L.DivIcon {
  const { color = '#22c55e', size = 44, pulse = true } = options;
  const svg = createTargetSvg({ color, size });

  const pulseRing = pulse
    ? `<div class="map-pin-pulse" style="
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        width: ${size}px;
        height: ${size}px;
        border-radius: 50%;
        background-color: ${color};
        opacity: 0.3;
        animation: map-pin-pulse 2s ease-out infinite;
        pointer-events: none;
      "></div>`
    : '';

  const html = `
    <div class="map-pin-container" style="position: relative;">
      ${pulseRing}
      ${svg}
    </div>
  `.trim();

  return L.divIcon({
    className: 'map-pin-icon',
    html,
    iconSize: [size, size],
    iconAnchor: [size / 2, size / 2], // Anchor at center
  });
}

/**
 * Creates a Leaflet DivIcon with a numbered circle.
 * Used in game summary map for round markers.
 */
export function createNumberedCircleIcon(
  L: typeof import('leaflet'),
  options: { number: number; color?: string; size?: number; textColor?: string }
): L.DivIcon {
  const { size = 28 } = options;
  const svg = createNumberedCircleSvg(options);

  return L.divIcon({
    className: 'map-pin-icon',
    html: svg,
    iconSize: [size, size],
    iconAnchor: [size / 2, size / 2], // Anchor at center
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

  @keyframes map-pin-glow {
    0%, 100% {
      opacity: 0.6;
      transform: translateX(-50%) scale(1);
    }
    50% {
      opacity: 1;
      transform: translateX(-50%) scale(1.15);
    }
  }
`;
