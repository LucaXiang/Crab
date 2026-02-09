import type { AuditDetailsRenderer, AuditDetailLine } from './types';

export const LoginSuccessRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.username) {
      lines.push({
        label: t('audit.detail.field.username'),
        value: String(details.username),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const EscalationSuccessRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.requester_name) {
      lines.push({
        label: t('audit.detail.field.requester_name'),
        value: String(details.requester_name),
      });
    }

    if (details.required_permission) {
      lines.push({
        label: t('audit.detail.field.required_permission'),
        value: String(details.required_permission),
      });
    }

    if (details.authorizer_username) {
      lines.push({
        label: t('audit.detail.field.authorizer_name'),
        value: String(details.authorizer_username),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const LoginFailedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.reason) {
      const reasonKey = `audit.detail.value.${details.reason}`;
      lines.push({
        label: t('audit.detail.field.reason'),
        value: t(reasonKey),
        valueClass: 'text-red-600',
      });
    }

    if (details.username) {
      lines.push({
        label: t('audit.detail.field.username'),
        value: String(details.username),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};
