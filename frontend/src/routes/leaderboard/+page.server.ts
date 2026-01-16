import { serverApi } from '$lib/server/api';
import type { PageServerLoad } from './$types';

interface LeaderboardEntry {
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  rank: number;
  score: number;
  games_played: number;
  is_current_user: boolean;
}

interface LeaderboardResponse {
  entries: LeaderboardEntry[];
  total_players: number;
  current_user_rank: number | null;
  current_user_score: number | null;
}

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
      currentUserScore: response.current_user_score
    };
  } catch (e) {
    console.error('Failed to load leaderboard:', e);
    return {
      initialEntries: [],
      totalPlayers: 0,
      currentUserRank: null,
      currentUserScore: null
    };
  }
};
