import React, { useEffect, useState } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import {
  Building2,
  CreditCard,
  Shield,
  FileKey,
  Loader2,
  Server,
  Monitor,
  Calendar,
  Clock,
  AlertCircle,
  CheckCircle2,
  XCircle,
  AlertTriangle,
} from 'lucide-react';

interface SubscriptionDetail {
  id: string | null;
  status: string;
  plan: string;
  starts_at: number;
  expires_at: number | null;
  max_stores: number;
  max_clients: number;
  features: string[];
  cancel_at_period_end: boolean;
  billing_interval: string | null;
  signature_valid_until: number;
  last_checked_at: number;
}

interface P12Detail {
  has_p12: boolean;
  fingerprint: string | null;
  subject: string | null;
  expires_at: number | null;
}

interface CertificateDetail {
  expires_at: number | null;
  days_remaining: number | null;
  fingerprint: string | null;
  issuer: string | null;
}

interface TenantDetails {
  tenant_id: string;
  mode: string | null;
  device_id: string;
  entity_id: string | null;
  entity_type: string | null;
  bound_at: number | null;
  subscription: SubscriptionDetail | null;
  p12: P12Detail | null;
  certificate: CertificateDetail | null;
}

function formatDate(ms: number | null | undefined): string {
  if (ms == null) return '-';
  return new Date(ms).toLocaleDateString(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function formatShortDate(ms: number | null | undefined): string {
  if (ms == null) return '-';
  return new Date(ms).toLocaleDateString(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
}

function isExpired(ms: number | null | undefined): boolean {
  if (ms == null) return false;
  return ms < Date.now();
}

function isExpiringSoon(ms: number | null | undefined, days: number = 30): boolean {
  if (ms == null) return false;
  const threshold = Date.now() + days * 24 * 60 * 60 * 1000;
  return ms < threshold && ms >= Date.now();
}

function truncateId(id: string, maxLen: number = 20): string {
  if (id.length <= maxLen) return id;
  return `${id.slice(0, 10)}...${id.slice(-6)}`;
}

const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
  const config: Record<string, { bg: string; text: string; icon: React.ReactNode }> = {
    active: { bg: 'bg-green-100', text: 'text-green-700', icon: <CheckCircle2 size={12} /> },
    past_due: { bg: 'bg-yellow-100', text: 'text-yellow-700', icon: <AlertTriangle size={12} /> },
    expired: { bg: 'bg-red-100', text: 'text-red-700', icon: <XCircle size={12} /> },
    canceled: { bg: 'bg-red-100', text: 'text-red-700', icon: <XCircle size={12} /> },
    unpaid: { bg: 'bg-red-100', text: 'text-red-700', icon: <AlertCircle size={12} /> },
    inactive: { bg: 'bg-gray-100', text: 'text-gray-600', icon: <AlertCircle size={12} /> },
  };
  const c = config[status] || config.inactive;
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${c.bg} ${c.text}`}>
      {c.icon}
      {status}
    </span>
  );
};

const PlanBadge: React.FC<{ plan: string }> = ({ plan }) => {
  const config: Record<string, { bg: string; text: string }> = {
    basic: { bg: 'bg-blue-100', text: 'text-blue-700' },
    pro: { bg: 'bg-purple-100', text: 'text-purple-700' },
    enterprise: { bg: 'bg-amber-100', text: 'text-amber-700' },
  };
  const c = config[plan] || config.basic;
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${c.bg} ${c.text} uppercase`}>
      {plan}
    </span>
  );
};

const InfoRow: React.FC<{ label: string; value: React.ReactNode; icon?: React.ReactNode; mono?: boolean }> = ({ label, value, icon, mono }) => (
  <div className="flex items-start justify-between py-2 gap-4">
    <span className="text-xs text-gray-500 flex items-center gap-1.5 shrink-0">
      {icon}
      {label}
    </span>
    <span className={`text-sm font-medium text-gray-800 text-right min-w-0 break-all ${mono ? 'font-mono text-xs' : ''}`}>{value}</span>
  </div>
);

const InfoCard: React.FC<{
  icon: React.ReactNode;
  iconBg: string;
  title: string;
  children: React.ReactNode;
}> = ({ icon, iconBg, title, children }) => (
  <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
    <div className="px-5 py-3 border-b border-gray-100 flex items-center gap-3">
      <div className={`w-8 h-8 ${iconBg} rounded-lg flex items-center justify-center`}>
        {icon}
      </div>
      <h3 className="text-sm font-bold text-gray-800">{title}</h3>
    </div>
    <div className="px-5 py-2 divide-y divide-gray-50">{children}</div>
  </div>
);

export const TenantInfoPanel: React.FC = () => {
  const { t } = useI18n();
  const [details, setDetails] = useState<TenantDetails | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    (async () => {
      try {
        const result = await invokeApi<TenantDetails | null>('get_tenant_details');
        setDetails(result);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  if (!details) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-400">
        <AlertCircle className="w-5 h-5 mr-2" />
        {t('settings.tenant_info.no_data')}
      </div>
    );
  }

  const { subscription: sub, p12, certificate: cert } = details;

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-indigo-50 rounded-lg flex items-center justify-center">
            <Building2 className="w-5 h-5 text-indigo-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('settings.tenant_info.title')}</h2>
            <p className="text-xs text-gray-500">{t('settings.tenant_info.description')}</p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {/* Device Info */}
        <InfoCard
          icon={<Server className="w-4 h-4 text-blue-600" />}
          iconBg="bg-blue-50"
          title={t('settings.tenant_info.device.title')}
        >
          <InfoRow
            label={t('settings.tenant_info.device.tenant_id')}
            value={truncateId(details.tenant_id)}
            mono
          />
          <InfoRow
            label={t('settings.tenant_info.device.mode')}
            value={
              details.mode ? (
                <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${
                  details.mode === 'Server' ? 'bg-blue-100 text-blue-700' : 'bg-teal-100 text-teal-700'
                }`}>
                  {details.mode === 'Server' ? <Server size={11} /> : <Monitor size={11} />}
                  {details.mode}
                </span>
              ) : '-'
            }
          />
          <InfoRow
            label={t('settings.tenant_info.device.device_id')}
            value={details.device_id}
            mono
          />
          {details.entity_id && (
            <InfoRow
              label={t('settings.tenant_info.device.entity_id')}
              value={truncateId(details.entity_id)}
              mono
            />
          )}
          {details.bound_at && (
            <InfoRow
              label={t('settings.tenant_info.device.bound_at')}
              value={formatDate(details.bound_at)}
              icon={<Calendar size={12} />}
            />
          )}
        </InfoCard>

        {/* Subscription */}
        <InfoCard
          icon={<CreditCard className="w-4 h-4 text-purple-600" />}
          iconBg="bg-purple-50"
          title={t('settings.tenant_info.subscription.title')}
        >
          {sub ? (
            <>
              <InfoRow
                label={t('settings.tenant_info.subscription.status')}
                value={<StatusBadge status={sub.status} />}
              />
              <InfoRow
                label={t('settings.tenant_info.subscription.plan')}
                value={
                  <span className="flex items-center gap-2">
                    <PlanBadge plan={sub.plan} />
                    {sub.billing_interval && (
                      <span className="text-xs text-gray-500">
                        {t(`settings.tenant_info.subscription.interval_${sub.billing_interval}`)}
                      </span>
                    )}
                  </span>
                }
              />
              <InfoRow
                label={t('settings.tenant_info.subscription.next_billing')}
                value={
                  sub.expires_at ? (
                    <span className={isExpired(sub.expires_at) ? 'text-red-600' : isExpiringSoon(sub.expires_at) ? 'text-yellow-600' : ''}>
                      {formatShortDate(sub.expires_at)}
                      {isExpired(sub.expires_at) && <span className="ml-1 text-xs text-red-500">({t('settings.tenant_info.expired')})</span>}
                    </span>
                  ) : (
                    <span className="text-gray-400">{t('settings.tenant_info.no_expiry')}</span>
                  )
                }
                icon={<Calendar size={12} />}
              />
              <InfoRow
                label={t('settings.tenant_info.subscription.max_stores')}
                value={sub.max_stores === 0 ? '∞' : sub.max_stores}
              />
              <InfoRow
                label={t('settings.tenant_info.subscription.max_clients')}
                value={sub.max_clients === 0 ? '∞' : sub.max_clients}
              />
              {sub.cancel_at_period_end && (
                <div className="flex items-center gap-2 py-2 px-3 mt-1 bg-yellow-50 border border-yellow-200 rounded-lg">
                  <AlertTriangle size={14} className="text-yellow-600 shrink-0" />
                  <span className="text-xs text-yellow-700">{t('settings.tenant_info.subscription.cancel_warning')}</span>
                </div>
              )}
            </>
          ) : (
            <div className="py-4 text-center text-xs text-gray-400">{t('settings.tenant_info.no_data')}</div>
          )}
        </InfoCard>

        {/* P12 Certificate */}
        <InfoCard
          icon={<FileKey className="w-4 h-4 text-amber-600" />}
          iconBg="bg-amber-50"
          title={t('settings.tenant_info.p12.title')}
        >
          {p12 ? (
            <>
              <InfoRow
                label={t('settings.tenant_info.p12.status')}
                value={
                  p12.has_p12 ? (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">
                      <CheckCircle2 size={12} />
                      {t('settings.tenant_info.p12.uploaded')}
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600">
                      <AlertCircle size={12} />
                      {t('settings.tenant_info.p12.not_uploaded')}
                    </span>
                  )
                }
              />
              {p12.subject && (
                <InfoRow label={t('settings.tenant_info.p12.subject')} value={p12.subject} />
              )}
              {p12.fingerprint && (
                <InfoRow
                  label={t('settings.tenant_info.p12.fingerprint')}
                  value={`${p12.fingerprint.slice(0, 16)}...`}
                  mono
                />
              )}
              {p12.expires_at && (
                <InfoRow
                  label={t('settings.tenant_info.p12.expires_at')}
                  value={
                    <span className={isExpired(p12.expires_at) ? 'text-red-600' : isExpiringSoon(p12.expires_at, 60) ? 'text-yellow-600' : ''}>
                      {formatShortDate(p12.expires_at)}
                      {isExpired(p12.expires_at) && <span className="ml-1 text-xs text-red-500">({t('settings.tenant_info.expired')})</span>}
                    </span>
                  }
                  icon={<Calendar size={12} />}
                />
              )}
            </>
          ) : (
            <div className="py-4 text-center text-xs text-gray-400">{t('settings.tenant_info.no_data')}</div>
          )}
        </InfoCard>

        {/* Device Certificate */}
        <InfoCard
          icon={<Shield className="w-4 h-4 text-green-600" />}
          iconBg="bg-green-50"
          title={t('settings.tenant_info.certificate.title')}
        >
          {cert ? (
            <>
              {cert.issuer && (
                <InfoRow
                  label={t('settings.tenant_info.certificate.common_name')}
                  value={truncateId(cert.issuer, 28)}
                  mono
                />
              )}
              <InfoRow
                label={t('settings.tenant_info.certificate.expires_at')}
                value={
                  cert.expires_at ? (
                    <span className={isExpired(cert.expires_at) ? 'text-red-600' : isExpiringSoon(cert.expires_at) ? 'text-yellow-600' : ''}>
                      {formatShortDate(cert.expires_at)}
                      {isExpired(cert.expires_at) && <span className="ml-1 text-xs text-red-500">({t('settings.tenant_info.expired')})</span>}
                    </span>
                  ) : '-'
                }
                icon={<Calendar size={12} />}
              />
              {cert.days_remaining != null && (
                <InfoRow
                  label={t('settings.tenant_info.certificate.days_remaining')}
                  value={
                    <span className={
                      cert.days_remaining < 0 ? 'text-red-600 font-bold' :
                      cert.days_remaining < 30 ? 'text-yellow-600' : 'text-green-600'
                    }>
                      {cert.days_remaining < 0
                        ? t('settings.tenant_info.expired')
                        : `${cert.days_remaining} ${t('settings.tenant_info.certificate.days')}`
                      }
                    </span>
                  }
                  icon={<Clock size={12} />}
                />
              )}
              {cert.fingerprint && (
                <InfoRow
                  label={t('settings.tenant_info.certificate.fingerprint')}
                  value={`${cert.fingerprint.slice(0, 16)}...`}
                  mono
                />
              )}
            </>
          ) : (
            <div className="py-4 text-center text-xs text-gray-400">{t('settings.tenant_info.no_data')}</div>
          )}
        </InfoCard>
      </div>
    </div>
  );
};
