const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:3001';

interface ApiError {
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
    
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
    };

    const options: RequestInit = {
      method,
      headers,
      credentials: 'include', // Send cookies
    };

    if (body) {
      options.body = JSON.stringify(body);
    }

    const response = await fetch(url, options);

    if (!response.ok) {
      const error: ApiError = await response.json().catch(() => ({
        code: 'UNKNOWN',
        message: 'An unknown error occurred',
      }));
      throw new ApiClientError(response.status, error.code, error.message);
    }

    // Handle 204 No Content
    if (response.status === 204) {
      return undefined as T;
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
