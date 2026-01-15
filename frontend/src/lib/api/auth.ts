import { api } from './client';

/**
 * User entity from the API.
 * All IDs use prefixed nanoid format (e.g., usr_V1StGXR8_Z5j)
 */
export interface User {
  /** User ID (prefixed nanoid: usr_xxxxxxxxxxxx) */
  id: string;
  /** Unique username (e.g., coolplayer42) */
  username: string | null;
  display_name: string;
  email: string | null;
  avatar_url: string | null;
  is_guest: boolean;
  games_played: number;
  total_score: number;
  best_score: number;
}

export const authApi = {
  /** Create a guest session */
  async createGuest(): Promise<User> {
    return api.post<User>('/auth/guest');
  },

  /** Get current authenticated user */
  async getCurrentUser(): Promise<User> {
    return api.get<User>('/auth/me');
  },

  /** Logout and destroy session */
  async logout(): Promise<void> {
    return api.post('/auth/logout');
  },

  /** Get Google OAuth URL */
  getGoogleAuthUrl(redirectTo?: string): string {
    const base = import.meta.env.VITE_API_URL || 'http://localhost:3001';
    const params = redirectTo ? `?redirect_to=${encodeURIComponent(redirectTo)}` : '';
    return `${base}/api/v1/auth/google${params}`;
  },

  /** Get Microsoft OAuth URL */
  getMicrosoftAuthUrl(redirectTo?: string): string {
    const base = import.meta.env.VITE_API_URL || 'http://localhost:3001';
    const params = redirectTo ? `?redirect_to=${encodeURIComponent(redirectTo)}` : '';
    return `${base}/api/v1/auth/microsoft${params}`;
  },
};
