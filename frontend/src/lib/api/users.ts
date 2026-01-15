import { api } from './client';
import type { User } from './auth';

/**
 * User profile (public view)
 */
export interface UserProfile {
  id: string;
  username: string | null;
  display_name: string;
  avatar_url: string | null;
  is_guest: boolean;
  games_played: number;
  total_score: number;
  best_score: number;
}

/**
 * Update profile request
 */
export interface UpdateProfileRequest {
  username?: string;
  display_name?: string;
  avatar_url?: string;
}

/**
 * Delete account response
 */
export interface DeleteAccountResponse {
  message: string;
}

/**
 * Session info
 */
export interface SessionInfo {
  id: string;
  is_current: boolean;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
  last_accessed_at: string;
  expires_at: string;
}

/**
 * Sessions list response
 */
export interface SessionsListResponse {
  sessions: SessionInfo[];
}

/**
 * Revoke session response
 */
export interface RevokeSessionResponse {
  message: string;
  revoked_count: number;
}

export const usersApi = {
  /** Get current user's profile */
  async getProfile(): Promise<UserProfile> {
    return api.get<UserProfile>('/users/me');
  },

  /** Update current user's profile */
  async updateProfile(data: UpdateProfileRequest): Promise<UserProfile> {
    return api.put<UserProfile>('/users/me', data);
  },

  /** Delete current user's account (soft delete) */
  async deleteAccount(): Promise<DeleteAccountResponse> {
    return api.delete<DeleteAccountResponse>('/users/me');
  },

  /** Get a user's public profile by ID */
  async getUserById(id: string): Promise<UserProfile> {
    return api.get<UserProfile>(`/users/${id}`);
  },

  /** Get a user's public profile by username */
  async getUserByUsername(username: string): Promise<UserProfile> {
    return api.get<UserProfile>(`/users/u/${username}`);
  },
};

export const sessionsApi = {
  /** List all active sessions */
  async listSessions(): Promise<SessionsListResponse> {
    return api.get<SessionsListResponse>('/sessions');
  },

  /** Revoke a specific session */
  async revokeSession(sessionId: string): Promise<RevokeSessionResponse> {
    return api.delete<RevokeSessionResponse>(`/sessions/${sessionId}`);
  },

  /** Revoke all other sessions (except current) */
  async revokeOtherSessions(): Promise<RevokeSessionResponse> {
    return api.delete<RevokeSessionResponse>('/sessions/others');
  },
};
