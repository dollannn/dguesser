import { browser } from '$app/environment';
import { authApi } from '$lib/api/auth';

const AUTH_REDIRECT_KEY = 'auth_redirect';

function normalizeRedirectTarget(target: string | null | undefined): string {
  if (!target || !target.startsWith('/')) {
    return '/';
  }

  if (
    target === '/auth' ||
    target.startsWith('/auth?') ||
    target.startsWith('/auth#') ||
    target === '/auth/success' ||
    target.startsWith('/auth/success?') ||
    target.startsWith('/auth/success#') ||
    target === '/auth/error' ||
    target.startsWith('/auth/error?') ||
    target.startsWith('/auth/error#')
  ) {
    return '/';
  }

  return target;
}

export function getCurrentRelativeUrl(): string {
  if (!browser) return '/';

  return `${window.location.pathname}${window.location.search}${window.location.hash}`;
}

export function setAuthRedirect(target?: string): string {
  const redirectTo = normalizeRedirectTarget(target ?? getCurrentRelativeUrl());

  if (browser) {
    sessionStorage.setItem(AUTH_REDIRECT_KEY, redirectTo);
  }

  return redirectTo;
}

export function getStoredAuthRedirect(): string {
  if (!browser) return '/';

  return normalizeRedirectTarget(sessionStorage.getItem(AUTH_REDIRECT_KEY));
}

export function clearStoredAuthRedirect(): void {
  if (!browser) return;

  sessionStorage.removeItem(AUTH_REDIRECT_KEY);
}

export function startGoogleAuth(target?: string): void {
  if (!browser) return;

  const redirectTo = setAuthRedirect(target);
  window.location.href = authApi.getGoogleAuthUrl(redirectTo);
}
