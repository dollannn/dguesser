// Socket.IO client module
// TODO: Implement in Phase 6

import { io, type Socket } from 'socket.io-client';

const REALTIME_URL = 'http://localhost:3002';

let socket: Socket | null = null;

export function getSocket(): Socket {
  if (!socket) {
    socket = io(REALTIME_URL, {
      autoConnect: false
    });
  }
  return socket;
}

export function connectSocket(): void {
  const s = getSocket();
  if (!s.connected) {
    s.connect();
  }
}

export function disconnectSocket(): void {
  if (socket?.connected) {
    socket.disconnect();
  }
}
