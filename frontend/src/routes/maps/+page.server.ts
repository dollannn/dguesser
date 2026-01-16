import { serverApi, ServerApiError } from '$lib/server/api';
import type { PageServerLoad } from './$types';

interface MapSummary {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: 'private' | 'unlisted' | 'public';
  is_system_map: boolean;
  is_owned: boolean;
  location_count: number;
  created_at: string;
}

interface ListMapsResponse {
  maps: MapSummary[];
}

export const load: PageServerLoad = async ({ request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const response = await serverApi.get<ListMapsResponse>('/maps', {
      cookies: cookieHeader
    });
    return {
      maps: response.maps,
      error: null
    };
  } catch (e) {
    console.error('Failed to load maps:', e);
    return {
      maps: [],
      error: e instanceof ServerApiError ? e.message : 'Failed to load maps'
    };
  }
};
