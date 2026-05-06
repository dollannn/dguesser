import { api } from './client';
import type { GameSettings } from './games';

export interface CreatePartyResponse {
  id: string;
  join_code: string;
}

export interface PartyMemberDetail {
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  is_host: boolean;
}

export interface PartyDetails {
  id: string;
  host_id: string;
  join_code: string;
  status: string;
  settings: Partial<GameSettings>;
  members: PartyMemberDetail[];
  created_at: string;
}

export interface ActivePartyDetails extends PartyDetails {
  current_game_id: string | null;
  phase: 'lobby' | 'in_game';
}

export interface JoinByCodeResponse {
  type: 'party' | 'game';
  id: string;
  join_code: string;
}

export const partiesApi = {
  /** Create a new party */
  async create(settings?: Partial<GameSettings>): Promise<CreatePartyResponse> {
    return api.post<CreatePartyResponse>('/parties', { settings });
  },

  /** Get the current user's active party, if any */
  async getActive(): Promise<ActivePartyDetails | null> {
    return api.get<ActivePartyDetails | null>('/parties/active');
  },

  /** Leave the current user's active party, if any */
  async leaveActive(): Promise<void> {
    return api.post<void>('/parties/active/leave');
  },

  /** Get party details */
  async get(partyId: string): Promise<PartyDetails> {
    return api.get<PartyDetails>(`/parties/${partyId}`);
  },

  /** Join by code (unified - returns party or game) */
  async joinByCode(code: string): Promise<JoinByCodeResponse> {
    return api.post<JoinByCodeResponse>('/parties/join', { code });
  },
};
