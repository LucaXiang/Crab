export interface TenantProfile {
  id: string;
  email: string;
  name: string | null;
  status: string;
  created_at: number;
}

export interface Subscription {
  id: string;
  status: string;
  plan: string;
  max_edge_servers: number;
  max_clients: number;
  current_period_end: number | null;
  cancel_at_period_end: boolean;
  billing_interval: string | null;
  created_at: number;
}

export interface P12Info {
  has_p12: boolean;
  fingerprint: string | null;
  subject: string | null;
  expires_at: number | null;
}
