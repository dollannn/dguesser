// Maps API client
import { api } from './client';

// =============================================================================
// Types
// =============================================================================

export type MapVisibility = 'private' | 'unlisted' | 'public';

export interface MapSummary {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: MapVisibility;
  is_system_map: boolean;
  is_owned: boolean;
  location_count: number;
  created_at: string;
}

export interface ListMapsResponse {
  maps: MapSummary[];
}

export interface CreateMapRequest {
  name: string;
  description?: string;
  visibility?: MapVisibility;
}

export interface CreateMapResponse {
  id: string;
  slug: string;
}

export interface MapDetails {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: MapVisibility;
  is_system_map: boolean;
  is_owned: boolean;
  is_default: boolean;
  location_count: number;
  created_at: string;
  updated_at: string;
}

export interface UpdateMapRequest {
  name?: string;
  description?: string;
  visibility?: MapVisibility;
}

export interface MapLocationItem {
  id: string;
  panorama_id: string;
  lat: number;
  lng: number;
  country_code: string | null;
  subdivision_code: string | null;
}

export interface MapLocationsResponse {
  locations: MapLocationItem[];
  total: number;
  page: number;
  per_page: number;
}

export interface AddLocationsRequest {
  location_ids: string[];
}

export interface AddLocationsResponse {
  added: number;
  total: number;
}

export interface AddLocationsFromUrlsRequest {
  urls: string[];
}

export interface UrlParseResult {
  url: string;
  success: boolean;
  error: string | null;
  location_id: string | null;
  already_exists: boolean;
}

export interface AddLocationsFromUrlsResponse {
  results: UrlParseResult[];
  added: number;
  total: number;
}

// Location search types
export interface LocationSearchFilters {
  country_code?: string;
  subdivision_code?: string;
  min_year?: number;
  max_year?: number;
  outdoor_only?: boolean;
  exclude_map_id?: string;
  page?: number;
  per_page?: number;
}

export interface LocationSearchItem {
  id: string;
  panorama_id: string;
  lat: number;
  lng: number;
  country_code: string | null;
  subdivision_code: string | null;
  capture_year: number | null;
}

export interface SearchLocationsResponse {
  locations: LocationSearchItem[];
  total: number;
  page: number;
  per_page: number;
}

export interface CountryInfo {
  code: string;
  count: number;
}

export interface CountriesResponse {
  countries: CountryInfo[];
}

export interface SubdivisionInfo {
  code: string;
  count: number;
}

export interface SubdivisionsResponse {
  subdivisions: SubdivisionInfo[];
}

// =============================================================================
// Maps API
// =============================================================================

export const mapsApi = {
  /**
   * List maps visible to the current user.
   */
  list(): Promise<ListMapsResponse> {
    return api.get<ListMapsResponse>('/maps');
  },

  /**
   * Get a map by ID.
   */
  get(id: string): Promise<MapDetails> {
    return api.get<MapDetails>(`/maps/${id}`);
  },

  /**
   * Create a new map.
   */
  create(data: CreateMapRequest): Promise<CreateMapResponse> {
    return api.post<CreateMapResponse>('/maps', data);
  },

  /**
   * Update a map.
   */
  update(id: string, data: UpdateMapRequest): Promise<MapDetails> {
    return api.put<MapDetails>(`/maps/${id}`, data);
  },

  /**
   * Delete a map.
   */
  delete(id: string): Promise<void> {
    return api.delete<void>(`/maps/${id}`);
  },

  /**
   * Get locations in a map.
   */
  getLocations(
    mapId: string,
    page = 1,
    perPage = 50
  ): Promise<MapLocationsResponse> {
    return api.get<MapLocationsResponse>(
      `/maps/${mapId}/locations?page=${page}&per_page=${perPage}`
    );
  },

  /**
   * Add locations to a map by IDs.
   */
  addLocations(
    mapId: string,
    locationIds: string[]
  ): Promise<AddLocationsResponse> {
    return api.post<AddLocationsResponse>(`/maps/${mapId}/locations`, {
      location_ids: locationIds,
    });
  },

  /**
   * Add locations from Street View URLs.
   */
  addLocationsFromUrls(
    mapId: string,
    urls: string[]
  ): Promise<AddLocationsFromUrlsResponse> {
    return api.post<AddLocationsFromUrlsResponse>(
      `/maps/${mapId}/locations/from-urls`,
      { urls }
    );
  },

  /**
   * Remove a location from a map.
   */
  removeLocation(mapId: string, locationId: string): Promise<void> {
    return api.delete<void>(`/maps/${mapId}/locations/${locationId}`);
  },
};

// =============================================================================
// Locations API
// =============================================================================

export const locationsApi = {
  /**
   * Search locations with filters.
   */
  search(filters: LocationSearchFilters = {}): Promise<SearchLocationsResponse> {
    const params = new URLSearchParams();

    if (filters.country_code) params.set('country_code', filters.country_code);
    if (filters.subdivision_code)
      params.set('subdivision_code', filters.subdivision_code);
    if (filters.min_year) params.set('min_year', filters.min_year.toString());
    if (filters.max_year) params.set('max_year', filters.max_year.toString());
    if (filters.outdoor_only) params.set('outdoor_only', 'true');
    if (filters.exclude_map_id)
      params.set('exclude_map_id', filters.exclude_map_id);
    if (filters.page) params.set('page', filters.page.toString());
    if (filters.per_page) params.set('per_page', filters.per_page.toString());

    const query = params.toString();
    return api.get<SearchLocationsResponse>(
      `/locations/search${query ? `?${query}` : ''}`
    );
  },

  /**
   * Get available countries for filtering.
   */
  getCountries(): Promise<CountriesResponse> {
    return api.get<CountriesResponse>('/locations/countries');
  },

  /**
   * Get available subdivisions for a country.
   */
  getSubdivisions(countryCode: string): Promise<SubdivisionsResponse> {
    return api.get<SubdivisionsResponse>(
      `/locations/countries/${countryCode}/subdivisions`
    );
  },
};
