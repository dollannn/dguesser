// Server-side API helper for SSR data fetching
import { env } from '$env/dynamic/private';

interface FetchOptions {
  cookies?: string;
}

class ServerApi {
  private get baseUrl(): string {
    return `${env.API_URL || 'http://localhost:3001'}/api/v1`;
  }

  async get<T>(path: string, options: FetchOptions = {}): Promise<T> {
    const headers: HeadersInit = {
      'Content-Type': 'application/json'
    };

    if (options.cookies) {
      headers['Cookie'] = options.cookies;
    }

    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'GET',
      headers
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({
        code: 'UNKNOWN',
        message: 'An unknown error occurred'
      }));
      throw new ServerApiError(response.status, error.code, error.message);
    }

    if (response.status === 204) {
      return undefined as T;
    }

    return response.json();
  }
}

export class ServerApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'ServerApiError';
  }
}

export const serverApi = new ServerApi();
