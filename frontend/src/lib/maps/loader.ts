let loadPromise: Promise<void> | null = null;

declare global {
  interface Window {
    google?: typeof google;
  }
}

export function loadGoogleMaps(): Promise<void> {
  if (loadPromise) return loadPromise;

  if (typeof window !== 'undefined' && window.google?.maps) {
    return Promise.resolve();
  }

  loadPromise = new Promise((resolve, reject) => {
    const apiKey = import.meta.env.VITE_GOOGLE_MAPS_API_KEY;

    if (!apiKey) {
      reject(new Error('Google Maps API key not configured'));
      return;
    }

    const script = document.createElement('script');
    script.src = `https://maps.googleapis.com/maps/api/js?key=${apiKey}&libraries=places`;
    script.async = true;
    script.defer = true;

    script.onload = () => resolve();
    script.onerror = () => reject(new Error('Failed to load Google Maps'));

    document.head.appendChild(script);
  });

  return loadPromise;
}

export function isGoogleMapsLoaded(): boolean {
  return typeof window !== 'undefined' && !!window.google?.maps;
}
