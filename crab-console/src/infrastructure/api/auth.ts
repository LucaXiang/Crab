import { request, requestFormData } from './client';

export interface LoginResponse {
  token: string;
  refresh_token: string;
  tenant_id: string;
}


export function login(email: string, password: string): Promise<LoginResponse> {
  return request('POST', '/api/tenant/login', { email, password });
}

export function forgotPassword(email: string): Promise<{ message: string }> {
  return request('POST', '/api/tenant/forgot-password', { email });
}

export function resetPassword(
  email: string,
  code: string,
  newPassword: string,
): Promise<{ message: string }> {
  return request('POST', '/api/tenant/reset-password', {
    email,
    code,
    new_password: newPassword,
  });
}

export function changePassword(
  token: string,
  currentPassword: string,
  newPassword: string,
): Promise<{ message: string }> {
  return request('POST', '/api/tenant/change-password', {
    current_password: currentPassword,
    new_password: newPassword,
  }, token);
}

export function changeEmail(
  token: string,
  currentPassword: string,
  newEmail: string,
): Promise<{ message: string }> {
  return request('POST', '/api/tenant/change-email', {
    current_password: currentPassword,
    new_email: newEmail,
  }, token);
}

export function confirmEmailChange(
  token: string,
  newEmail: string,
  code: string,
): Promise<{ message: string }> {
  return request('POST', '/api/tenant/confirm-email-change', {
    new_email: newEmail,
    code,
  }, token);
}

export interface P12UploadResponse {
  success: boolean;
  fingerprint: string;
  common_name: string;
  organization: string | null;
  tax_id: string | null;
  issuer: string;
  expires_at: number;
}

export async function uploadP12(
  token: string,
  p12File: File,
  p12Password: string,
): Promise<P12UploadResponse> {
  const form = new FormData();
  form.append('p12_password', p12Password);
  form.append('p12_file', p12File);
  return requestFormData('/api/p12/upload', form, token);
}
