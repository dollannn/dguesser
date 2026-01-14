import { browser } from '$app/environment';
import { loadGoogleMaps } from '$lib/maps/loader';

export async function load() {
  // Pre-load Google Maps API on client side
  if (browser) {
    await loadGoogleMaps();
  }
  return {};
}
