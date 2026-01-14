<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';

  interface Guess {
    lat: number;
    lng: number;
    displayName: string;
  }

  interface Props {
    correctLat: number;
    correctLng: number;
    guesses: Guess[];
  }

  let { correctLat, correctLng, guesses }: Props = $props();

  let container: HTMLDivElement;
  let map: google.maps.Map | null = null;

  onMount(() => {
    if (!browser) return;

    // Initialize map
    map = new google.maps.Map(container, {
      center: { lat: correctLat, lng: correctLng },
      zoom: 4,
      disableDefaultUI: true,
      zoomControl: true,
      gestureHandling: 'greedy',
    });

    // Add correct location marker
    new google.maps.Marker({
      position: { lat: correctLat, lng: correctLng },
      map,
      icon: {
        path: google.maps.SymbolPath.CIRCLE,
        scale: 12,
        fillColor: '#ef4444',
        fillOpacity: 1,
        strokeColor: '#ffffff',
        strokeWeight: 3,
      },
      title: 'Correct Location',
    });

    // Add guess markers and lines
    const bounds = new google.maps.LatLngBounds();
    bounds.extend({ lat: correctLat, lng: correctLng });

    const colors = ['#22c55e', '#3b82f6', '#a855f7', '#f59e0b', '#ec4899', '#06b6d4'];

    guesses.forEach((guess, i) => {
      const color = colors[i % colors.length];

      // Guess marker
      new google.maps.Marker({
        position: { lat: guess.lat, lng: guess.lng },
        map,
        icon: {
          path: google.maps.SymbolPath.CIRCLE,
          scale: 8,
          fillColor: color,
          fillOpacity: 1,
          strokeColor: '#ffffff',
          strokeWeight: 2,
        },
        title: guess.displayName,
      });

      // Line from guess to correct location
      new google.maps.Polyline({
        path: [
          { lat: guess.lat, lng: guess.lng },
          { lat: correctLat, lng: correctLng },
        ],
        geodesic: true,
        strokeColor: color,
        strokeOpacity: 0.8,
        strokeWeight: 2,
        map,
      });

      bounds.extend({ lat: guess.lat, lng: guess.lng });
    });

    // Fit map to show all markers
    if (guesses.length > 0) {
      map.fitBounds(bounds, 50);
    }
  });
</script>

<div bind:this={container} class="w-full h-full"></div>
