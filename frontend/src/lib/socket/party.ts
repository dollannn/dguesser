import { writable, get } from 'svelte/store';
import { goto } from '$app/navigation';
import { socketClient, toastStore } from './client';
import type { GameSettings } from '$lib/api/games';

// =============================================================================
// Types matching backend protocol
// =============================================================================

export interface PartyMemberInfo {
  user_id: string;
  display_name: string;
  avatar_url: string | null;
  connected: boolean;
}

export interface PartyStatePayload {
  party_id: string;
  join_code: string;
  host_id: string;
  members: PartyMemberInfo[];
  settings: GameSettings;
  current_game_id: string | null;
  phase: 'lobby' | 'in_game';
}

export interface PartyMemberJoinedPayload {
  member: PartyMemberInfo;
}

export interface PartyMemberLeftPayload {
  user_id: string;
  display_name: string;
}

export interface PartyGameStartingPayload {
  game_id: string;
}

export interface PartyGameEndedPayload {
  game_id: string;
}

export interface PartyDisbandedPayload {
  reason: string;
}

export interface PartyHostChangedPayload {
  new_host_id: string;
  new_host_name: string;
}

export interface PartySettingsUpdatedPayload {
  settings: GameSettings;
}

// =============================================================================
// Store
// =============================================================================

export interface PartyState {
  partyId: string | null;
  joinCode: string | null;
  hostId: string | null;
  members: Map<string, PartyMemberInfo>;
  settings: GameSettings | null;
  status: 'idle' | 'lobby' | 'in_game';
  currentGameId: string | null;
}

const initialState: PartyState = {
  partyId: null,
  joinCode: null,
  hostId: null,
  members: new Map(),
  settings: null,
  status: 'idle',
  currentGameId: null,
};

function createPartyStore() {
  const { subscribe, set, update } = writable<PartyState>({ ...initialState });

  return {
    subscribe,

    /** Join a party via socket */
    async joinParty(partyId: string) {
      await socketClient.waitForAuth();
      socketClient.emit('party:join', { party_id: partyId });
    },

    /** Leave the current party */
    leaveParty() {
      const state = get({ subscribe });
      if (state.partyId) {
        socketClient.emit('party:leave', { party_id: state.partyId });
      }
      set({ ...initialState });
    },

    /** Create a new party via socket */
    async createParty(settings?: Partial<GameSettings>) {
      await socketClient.waitForAuth();
      socketClient.emit('party:create', { settings: settings || null });
    },

    /** Start a game from the party (host only) */
    startGame() {
      const state = get({ subscribe });
      if (state.partyId) {
        socketClient.emit('party:start_game', { party_id: state.partyId });
      }
    },

    /** Update party settings (host only) */
    updateSettings(settings: GameSettings) {
      const state = get({ subscribe });
      if (state.partyId) {
        socketClient.emit('party:update_settings', {
          party_id: state.partyId,
          settings,
        });
      }
    },

    /** Kick a member (host only) */
    kickMember(userId: string) {
      const state = get({ subscribe });
      if (state.partyId) {
        socketClient.emit('party:kick', {
          party_id: state.partyId,
          user_id: userId,
        });
      }
    },

    /** Disband the party (host only) */
    disbandParty() {
      const state = get({ subscribe });
      if (state.partyId) {
        socketClient.emit('party:disband', { party_id: state.partyId });
      }
    },

    /** Reset store to initial state */
    reset() {
      set({ ...initialState });
    },

    // =========================================================================
    // Event Handlers (called by initPartySocketListeners)
    // =========================================================================

    handlePartyState(payload: PartyStatePayload) {
      const members = new Map<string, PartyMemberInfo>();
      for (const m of payload.members) {
        members.set(m.user_id, m);
      }

      set({
        partyId: payload.party_id,
        joinCode: payload.join_code,
        hostId: payload.host_id,
        members,
        settings: payload.settings,
        status: payload.phase === 'in_game' ? 'in_game' : 'lobby',
        currentGameId: payload.current_game_id,
      });
    },

    handleMemberJoined(payload: PartyMemberJoinedPayload) {
      update((state) => {
        state.members.set(payload.member.user_id, payload.member);
        return state;
      });
    },

    handleMemberLeft(payload: PartyMemberLeftPayload) {
      update((state) => {
        state.members.delete(payload.user_id);
        return state;
      });
    },

    handleGameStarting(payload: PartyGameStartingPayload) {
      update((state) => ({
        ...state,
        status: 'in_game' as const,
        currentGameId: payload.game_id,
      }));

      // Auto-navigate to the game
      goto(`/game/${payload.game_id}`);
    },

    handleGameEnded(payload: PartyGameEndedPayload) {
      const state = get({ subscribe });
      update((s) => ({
        ...s,
        status: 'lobby' as const,
        currentGameId: null,
      }));

      // Don't auto-navigate if the user is still looking at the game results.
      // The GameFinished component will show a "Return to Party" button.
      // But if the user is somehow on a different page, navigate them back.
    },

    handleDisbanded(payload: PartyDisbandedPayload) {
      set({ ...initialState });
      toastStore.add('info', `Party disbanded: ${payload.reason}`);
      goto('/play');
    },

    handleHostChanged(payload: PartyHostChangedPayload) {
      update((state) => ({
        ...state,
        hostId: payload.new_host_id,
      }));
      toastStore.add('info', `${payload.new_host_name} is now the party host`);
    },

    handleSettingsUpdated(payload: PartySettingsUpdatedPayload) {
      update((state) => ({
        ...state,
        settings: payload.settings,
      }));
    },

    handleKicked() {
      set({ ...initialState });
      toastStore.add('error', 'You were kicked from the party');
      goto('/play');
    },
  };
}

export const partyStore = createPartyStore();

// =============================================================================
// Socket Listeners
// =============================================================================

export function initPartySocketListeners() {
  socketClient.on('party:state', (payload: PartyStatePayload) => {
    partyStore.handlePartyState(payload);
  });

  socketClient.on('party:member_joined', (payload: PartyMemberJoinedPayload) => {
    partyStore.handleMemberJoined(payload);
  });

  socketClient.on('party:member_left', (payload: PartyMemberLeftPayload) => {
    partyStore.handleMemberLeft(payload);
  });

  socketClient.on('party:game_starting', (payload: PartyGameStartingPayload) => {
    partyStore.handleGameStarting(payload);
  });

  socketClient.on('party:game_ended', (payload: PartyGameEndedPayload) => {
    partyStore.handleGameEnded(payload);
  });

  socketClient.on('party:disbanded', (payload: PartyDisbandedPayload) => {
    partyStore.handleDisbanded(payload);
  });

  socketClient.on('party:host_changed', (payload: PartyHostChangedPayload) => {
    partyStore.handleHostChanged(payload);
  });

  socketClient.on('party:settings_updated', (payload: PartySettingsUpdatedPayload) => {
    partyStore.handleSettingsUpdated(payload);
  });

  socketClient.on('party:kicked', () => {
    partyStore.handleKicked();
  });

  socketClient.on('party:error', (payload: { code: string; message: string }) => {
    toastStore.add('error', payload.message);
  });
}
