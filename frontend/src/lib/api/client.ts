/** Base URL for the backend API, configurable via VITE_API_URL env var */
export const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3001';

interface ApiErrorBody {
  code: string;
  message: string;
}

class ApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }

  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;

    const headers: HeadersInit = {};
    const options: RequestInit = {
      method,
      headers,
      credentials: 'include', // Send cookies
    };

    if (body) {
      headers['Content-Type'] = 'application/json';
      options.body = JSON.stringify(body);
    }

    let response: Response;
    try {
      response = await fetch(url, options);
    } catch (err) {
      // Wrap network errors (DNS failure, offline, CORS) in ApiClientError
      const message = err instanceof Error ? err.message : 'Network request failed';
      throw new ApiClientError(0, 'NETWORK_ERROR', message);
    }

    if (!response.ok) {
      const error: ApiErrorBody = await response.json().catch(() => ({
        code: 'UNKNOWN',
        message: response.statusText || 'An unknown error occurred',
      }));
      throw new ApiClientError(response.status, error.code, error.message);
    }

    // Handle 204 No Content — callers expecting void will get undefined
    if (response.status === 204) {
      return undefined as unknown as T;
    }

    return response.json();
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  put<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PUT', path, body);
  }

  patch<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('PATCH', path, body);
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>('DELETE', path);
  }
}

export class ApiClientError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string
  ) {
    super(message);
    this.name = 'ApiClientError';
  }
}

export const api = new ApiClient(`${API_BASE}/api/v1`);
