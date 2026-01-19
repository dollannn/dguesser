import { serverApi, ServerApiError } from '$lib/server/api';
import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

interface UserProfile {
  id: string;
  username: string;
  display_name: string;
  avatar_url: string | null;
  is_guest: boolean;
  games_played: number;
  total_score: number;
  best_score: number;
}

export const load: PageServerLoad = async ({ params, request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const profile = await serverApi.get<UserProfile>(
      `/users/u/${params.username}`,
      { cookies: cookieHeader }
    );
    return { profile };
  } catch (e) {
    if (e instanceof ServerApiError && e.status === 404) {
      throw error(404, 'User not found');
    }
    throw error(500, 'Failed to load profile');
  }
};
