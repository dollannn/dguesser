import { error } from '@sveltejs/kit';

import { serverApi, ServerApiError } from '$lib/server/api';
import type { UserProfile } from '$lib/api/users';
import type { PageServerLoad } from './$types';

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
