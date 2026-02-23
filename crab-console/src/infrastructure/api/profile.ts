import { request } from './client';
import type { TenantProfile, Subscription, P12Info } from '@/core/types/auth';

export interface ProfileResponse {
  profile: TenantProfile;
  subscription: Subscription | null;
  p12: P12Info | null;
}

export function getProfile(token: string): Promise<ProfileResponse> {
  return request('GET', '/api/tenant/profile', undefined, token);
}

export function updateProfile(token: string, name: string): Promise<{ message: string }> {
  return request('PUT', '/api/tenant/profile', { name }, token);
}

export function createBillingPortal(token: string): Promise<{ url: string }> {
  return request('POST', '/api/tenant/billing-portal', undefined, token);
}

export function createCheckout(token: string, plan: string): Promise<{ checkout_url: string }> {
  return request('POST', '/api/tenant/create-checkout', { plan }, token);
}
