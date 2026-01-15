import { api } from './client';

// =============================================================================
// Types
// =============================================================================

export interface AdminStats {
  total_locations: number;
  active_locations: number;
  pending_review: number;
  recent_reports: number;
  by_status: Record<string, number>;
  by_source: Record<string, number>;
  by_review_status: Record<string, number>;
}

export interface ReviewQueueItem {
  id: string;
  panorama_id: string;
  lat: number;
  lng: number;
  country_code: string | null;
  failure_count: number;
  report_count: number;
  last_report_reason: string | null;
  review_status: string;
  created_at: string;
}

export interface ReviewQueueResponse {
  locations: ReviewQueueItem[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface LocationReport {
  id: string;
  location_id: string;
  user_id: string | null;
  reason: string;
  notes: string | null;
  created_at: string;
}

export interface LocationDetail {
  id: string;
  panorama_id: string;
  lat: number;
  lng: number;
  country_code: string | null;
  subdivision_code: string | null;
  capture_date: string | null;
  provider: string;
  active: boolean;
  validation_status: string;
  source: string;
  surface: string | null;
  arrow_count: number | null;
  is_scout: boolean;
  buildings_100: number | null;
  roads_100: number | null;
  elevation: number | null;
  heading: number | null;
  failure_count: number;
  last_failure_reason: string | null;
  review_status: string;
  reviewed_at: string | null;
  reviewed_by: string | null;
  created_at: string;
  reports: LocationReport[];
}

export interface ReportWithLocation {
  id: string;
  location_id: string;
  panorama_id: string;
  lat: number;
  lng: number;
  country_code: string | null;
  user_id: string | null;
  reason: string;
  notes: string | null;
  created_at: string;
  location_review_status: string;
}

export interface ReportsListResponse {
  reports: ReportWithLocation[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface UpdateReviewStatusResponse {
  message: string;
  status: string;
  active: boolean;
}

export type ReviewStatus = 'approved' | 'rejected' | 'flagged' | 'pending';

// =============================================================================
// API Client
// =============================================================================

export const adminApi = {
  /** Get admin dashboard statistics */
  async getStats(): Promise<AdminStats> {
    return api.get<AdminStats>('/admin/stats');
  },

  /** Get paginated review queue */
  async getReviewQueue(params?: {
    page?: number;
    per_page?: number;
    status?: string;
  }): Promise<ReviewQueueResponse> {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', String(params.page));
    if (params?.per_page) searchParams.set('per_page', String(params.per_page));
    if (params?.status) searchParams.set('status', params.status);

    const query = searchParams.toString();
    const path = query ? `/admin/locations/review-queue?${query}` : '/admin/locations/review-queue';
    return api.get<ReviewQueueResponse>(path);
  },

  /** Get detailed location information */
  async getLocationDetail(locationId: string): Promise<LocationDetail> {
    return api.get<LocationDetail>(`/admin/locations/${locationId}`);
  },

  /** Update location review status */
  async updateReviewStatus(
    locationId: string,
    status: ReviewStatus,
    notes?: string
  ): Promise<UpdateReviewStatusResponse> {
    return api.put<UpdateReviewStatusResponse>(`/admin/locations/${locationId}/review`, {
      status,
      notes,
    });
  },

  /** Get paginated reports list */
  async getReports(params?: {
    page?: number;
    per_page?: number;
    reason?: string;
    location_status?: string;
  }): Promise<ReportsListResponse> {
    const searchParams = new URLSearchParams();
    if (params?.page) searchParams.set('page', String(params.page));
    if (params?.per_page) searchParams.set('per_page', String(params.per_page));
    if (params?.reason) searchParams.set('reason', params.reason);
    if (params?.location_status) searchParams.set('location_status', params.location_status);

    const query = searchParams.toString();
    const path = query ? `/admin/reports?${query}` : '/admin/reports';
    return api.get<ReportsListResponse>(path);
  },
};
