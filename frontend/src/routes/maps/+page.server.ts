import { serverApi, ServerApiError } from '$lib/server/api';
import type { ListMapsResponse } from '$lib/api/maps';
import type { PageServerLoad } from './$types';

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
