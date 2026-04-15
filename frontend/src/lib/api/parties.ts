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

  /** Get party details */
  async get(partyId: string): Promise<PartyDetails> {
    return api.get<PartyDetails>(`/parties/${partyId}`);
  },

  /** Join by code (unified - returns party or game) */
  async joinByCode(code: string): Promise<JoinByCodeResponse> {
    return api.post<JoinByCodeResponse>('/parties/join', { code });
  },
};
