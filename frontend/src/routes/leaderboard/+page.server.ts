import { serverApi } from '$lib/server/api';
import type { LeaderboardResponse } from '$lib/api/leaderboard';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ request }) => {
  try {
    const cookieHeader = request.headers.get('cookie') || '';
    const response = await serverApi.get<LeaderboardResponse>(
      '/leaderboard?type=total_score&period=all_time&limit=50',
      { cookies: cookieHeader }
    );
    return {
      initialEntries: response.entries,
      totalPlayers: response.total_players,
      currentUserRank: response.current_user_rank,
      currentUserScore: response.current_user_score,
      ssrError: false
    };
  } catch (e) {
    console.error('Failed to load leaderboard:', e);
    return {
      initialEntries: [],
      totalPlayers: 0,
      currentUserRank: null,
      currentUserScore: null,
      ssrError: true
    };
  }
};
