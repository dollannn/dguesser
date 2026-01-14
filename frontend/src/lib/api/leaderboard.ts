import { api } from './client';

export type LeaderboardType = 'total_score' | 'best_game' | 'games_played' | 'average_score';
export type TimePeriod = 'all_time' | 'daily' | 'weekly' | 'monthly';

export interface LeaderboardEntry {
  rank: number;
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  score: number;
  games_played: number;
  is_current_user: boolean;
}

export interface LeaderboardResponse {
  leaderboard_type: LeaderboardType;
  time_period: TimePeriod;
  entries: LeaderboardEntry[];
  current_user_rank: number | null;
  current_user_score: number | null;
  total_players: number;
}

export interface LeaderboardQuery {
  type?: LeaderboardType;
  period?: TimePeriod;
  limit?: number;
  offset?: number;
}

export const leaderboardApi = {
  /**
   * Get leaderboard data
   */
  async getLeaderboard(query: LeaderboardQuery = {}): Promise<LeaderboardResponse> {
    const params = new URLSearchParams();
    if (query.type) params.set('type', query.type);
    if (query.period) params.set('period', query.period);
    if (query.limit) params.set('limit', query.limit.toString());
    if (query.offset) params.set('offset', query.offset.toString());

    const queryString = params.toString();
    const path = `/leaderboard${queryString ? `?${queryString}` : ''}`;
    return api.get<LeaderboardResponse>(path);
  },
};
