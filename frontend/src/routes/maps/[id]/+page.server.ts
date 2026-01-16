import { serverApi, ServerApiError } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

interface MapDetails {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  visibility: 'private' | 'unlisted' | 'public';
  is_system_map: boolean;
  is_owned: boolean;
  is_default: boolean;
  location_count: number;
  created_at: string;
  updated_at: string;
}

export const load: PageServerLoad = async ({ params, request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const map = await serverApi.get<MapDetails>(`/maps/${params.id}`, {
      cookies: cookieHeader
    });
    return { map };
  } catch (e) {
    if (e instanceof ServerApiError) {
      if (e.status === 404) {
        throw error(404, 'Map not found');
      }
      if (e.status === 403) {
        throw error(403, 'You do not have access to this map');
      }
    }
    throw error(500, 'Failed to load map');
  }
};
