import React, { useState } from 'react';
import type { ArchivedOrderDetail } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { FileUp, X, User, MapPin, Mail, Phone } from 'lucide-react';

interface UpgradeInvoiceModalProps {
  order: ArchivedOrderDetail;
  onClose: () => void;
  onCreated: () => void;
}

export const UpgradeInvoiceModal: React.FC<UpgradeInvoiceModalProps> = ({ order, onClose, onCreated }) => {
  const { t } = useI18n();
  const [customerNif, setCustomerNif] = useState('');
  const [customerNombre, setCustomerNombre] = useState('');
  const [customerAddress, setCustomerAddress] = useState('');
  const [customerEmail, setCustomerEmail] = useState('');
  const [customerPhone, setCustomerPhone] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const canSubmit = customerNif.trim().length > 0 && customerNombre.trim().length > 0;

  const handleSubmit = async () => {
    if (!canSubmit || submitting) return;
    setSubmitting(true);
    try {
      await invokeApi('create_upgrade', {
        request: {
          order_pk: order.order_id,
          customer_nif: customerNif.trim(),
          customer_nombre: customerNombre.trim(),
          customer_address: customerAddress.trim() || null,
          customer_email: customerEmail.trim() || null,
          customer_phone: customerPhone.trim() || null,
        },
      });
      toast.success(t('upgrade.success'));
      onCreated();
    } catch (error) {
      toast.error(getErrorMessage(error));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-lg mx-4" onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-100">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center">
              <FileUp className="text-blue-600" size={20} />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">{t('upgrade.modal.title')}</h2>
              <p className="text-sm text-gray-500">{order.receipt_number}</p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full transition-colors">
            <X size={20} className="text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <div className="p-5 space-y-4">
          <p className="text-sm text-gray-500">{t('upgrade.modal.description')}</p>

          {/* NIF (required) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              NIF <span className="text-red-500">*</span>
            </label>
            <div className="relative">
              <User size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
              <input
                type="text"
                value={customerNif}
                onChange={e => setCustomerNif(e.target.value)}
                placeholder="B12345678"
                className="w-full pl-9 pr-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-200 focus:border-blue-400"
                autoFocus
              />
            </div>
          </div>

          {/* Nombre (required) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('upgrade.field.nombre')} <span className="text-red-500">*</span>
            </label>
            <input
              type="text"
              value={customerNombre}
              onChange={e => setCustomerNombre(e.target.value)}
              placeholder={t('upgrade.field.nombre_placeholder')}
              className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-200 focus:border-blue-400"
            />
          </div>

          {/* Address (optional) */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('upgrade.field.address')}
            </label>
            <div className="relative">
              <MapPin size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
              <input
                type="text"
                value={customerAddress}
                onChange={e => setCustomerAddress(e.target.value)}
                placeholder={t('upgrade.field.address_placeholder')}
                className="w-full pl-9 pr-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-200 focus:border-blue-400"
              />
            </div>
          </div>

          {/* Email + Phone row */}
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('upgrade.field.email')}
              </label>
              <div className="relative">
                <Mail size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                <input
                  type="email"
                  value={customerEmail}
                  onChange={e => setCustomerEmail(e.target.value)}
                  placeholder="email@example.com"
                  className="w-full pl-9 pr-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-200 focus:border-blue-400"
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {t('upgrade.field.phone')}
              </label>
              <div className="relative">
                <Phone size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                <input
                  type="tel"
                  value={customerPhone}
                  onChange={e => setCustomerPhone(e.target.value)}
                  placeholder="+34 600 000 000"
                  className="w-full pl-9 pr-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-200 focus:border-blue-400"
                />
              </div>
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-3 p-5 border-t border-gray-100">
          <button
            onClick={onClose}
            className="px-4 py-2.5 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-lg hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSubmit}
            disabled={!canSubmit || submitting}
            className="px-4 py-2.5 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
          >
            {submitting && <div className="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent" />}
            <FileUp size={16} />
            <span>{t('upgrade.modal.submit')}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
