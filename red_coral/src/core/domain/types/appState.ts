/**
 * 应用状态类型定义
 *
 * 与 Rust 定义保持一致: shared/src/app_state.rs
 */

// =============================================================================
// 激活失败原因
// =============================================================================

export type ClockDirection = 'backward' | 'forward';

export type ActivationRequiredReason =
  | { code: 'FirstTimeSetup' }
  | { code: 'CertificateExpired'; details: { expired_at: number; days_overdue: number } }
  | { code: 'CertificateExpiringSoon'; details: { expires_at: number; days_remaining: number } }
  | { code: 'CertificateInvalid'; details: { error: string } }
  | { code: 'SignatureInvalid'; details: { component: string; error: string } }
  | { code: 'DeviceMismatch'; details: { expected: string; actual: string } }
  | {
      code: 'ClockTampering';
      details: { direction: ClockDirection; drift_seconds: number; last_verified_at: number };
    }
  | { code: 'BindingInvalid'; details: { error: string } }
  | { code: 'TokenExpired'; details: { expired_at: number } }
  | { code: 'NetworkError'; details: { error: string; can_continue_offline: boolean } }
  | { code: 'Revoked'; details: { revoked_at: number; reason: string } };

// =============================================================================
// 订阅阻止信息
// =============================================================================

export type SubscriptionStatus = 'inactive' | 'active' | 'past_due' | 'expired' | 'canceled' | 'unpaid';
export type PlanType = 'basic' | 'pro' | 'enterprise';

export interface SubscriptionBlockedInfo {
  status: SubscriptionStatus;
  plan: PlanType;
  /** Plan 允许的最大门店数，0 = 无限 */
  max_stores: number;
  expired_at?: number;
  grace_period_days?: number;
  grace_period_ends_at?: number;
  in_grace_period: boolean;
  support_url?: string;
  renewal_url?: string;
  user_message: string;
}

// =============================================================================
// 健康检查
// =============================================================================

export type HealthLevel = 'healthy' | 'warning' | 'critical' | 'unknown';

export interface CertificateHealth {
  status: HealthLevel;
  expires_at?: number;
  days_remaining?: number;
  fingerprint?: string;
  issuer?: string;
}

export interface SubscriptionHealth {
  status: HealthLevel;
  plan?: string;
  subscription_status?: string;
  signature_valid_until: number;
  needs_refresh: boolean;
}

export interface NetworkHealth {
  status: HealthLevel;
  auth_server_reachable: boolean;
  last_connected_at?: number;
}

export interface DatabaseHealth {
  status: HealthLevel;
  size_bytes?: number;
  last_write_at?: number;
}

export interface ComponentsHealth {
  certificate: CertificateHealth;
  subscription: SubscriptionHealth;
  network: NetworkHealth;
  database: DatabaseHealth;
}

export interface DeviceInfo {
  device_id: string;
  entity_id?: string;
  tenant_id?: string;
}

export interface HealthStatus {
  overall: HealthLevel;
  components: ComponentsHealth;
  checked_at: number;
  device_info: DeviceInfo;
}

// =============================================================================
// AppState
// =============================================================================

export type AppState =
  // 前置状态 (未选模式)
  | { type: 'NeedTenantLogin' }
  | { type: 'TenantReady' }
  // Server 模式
  | {
      type: 'ServerNeedActivation';
      data: { reason: ActivationRequiredReason; can_auto_recover: boolean; recovery_hint: string };
    }
  | { type: 'ServerSubscriptionBlocked'; data: { info: SubscriptionBlockedInfo } }
  | { type: 'ServerReady' }
  | { type: 'ServerAuthenticated' }
  // Client 模式
  | {
      type: 'ClientNeedActivation';
      data: { reason: ActivationRequiredReason; can_auto_recover: boolean; recovery_hint: string };
    }
  | { type: 'ClientDisconnected' }
  | { type: 'ClientConnected' }
  | { type: 'ClientAuthenticated' };

// =============================================================================
// 辅助函数
// =============================================================================

/** 获取激活原因的用户友好消息 */
export function getActivationReasonMessage(reason: ActivationRequiredReason): string {
  switch (reason.code) {
    case 'FirstTimeSetup':
      return '欢迎！请激活您的设备';
    case 'CertificateExpired':
      return `设备证书已过期 ${reason.details.days_overdue} 天`;
    case 'CertificateExpiringSoon':
      return `证书将在 ${reason.details.days_remaining} 天后过期`;
    case 'CertificateInvalid':
      return '证书文件损坏或无效';
    case 'SignatureInvalid':
      return '安全验证失败';
    case 'DeviceMismatch':
      return '检测到硬件变更';
    case 'ClockTampering':
      return reason.details.direction === 'backward'
        ? `系统时间异常：回拨了 ${Math.floor(reason.details.drift_seconds / 3600)} 小时`
        : `系统时间异常：前跳了 ${Math.floor(reason.details.drift_seconds / 86400)} 天`;
    case 'BindingInvalid':
      return '设备绑定无效';
    case 'TokenExpired':
      return '凭据已过期';
    case 'NetworkError':
      return '无法连接服务器';
    case 'Revoked':
      return '此设备已被停用';
    default:
      return '需要重新激活';
  }
}
