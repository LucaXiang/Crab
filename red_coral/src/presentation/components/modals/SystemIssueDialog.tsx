/**
 * System Issue Dialog
 *
 * 渲染协议驱动的系统问题对话框。
 * - blocking=true → 全屏遮罩，不可关闭
 * - blocking=false → 可关闭的通知弹窗
 * - 使用 kind 作为 i18n key 查找标题/描述，找不到时 fallback 到 issue 字段
 * - 选项按钮由 options[] 驱动，选择"其他"时弹出文本输入
 */

import React, { useState } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import type { SystemIssue, ResolveSystemIssueRequest } from '@/core/domain/types/api';

interface SystemIssueDialogProps {
  issue: SystemIssue | null;
  onResolve: (data: ResolveSystemIssueRequest) => Promise<void>;
}

export const SystemIssueDialog: React.FC<SystemIssueDialogProps> = ({ issue, onResolve }) => {
  const { t } = useI18n();
  const [selectedOption, setSelectedOption] = useState<string | null>(null);
  const [customInput, setCustomInput] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!issue) return null;

  // i18n: try kind-based lookup first, fall back to issue text fields
  const kindTitle = t(`system_issue.kind.${issue.kind}.title`, { defaultValue: '', ...issue.params });
  const kindDesc = t(`system_issue.kind.${issue.kind}.description`, { defaultValue: '', ...issue.params });
  const title = kindTitle || issue.title || issue.kind;
  const description = kindDesc || issue.description || '';

  // Translate each option via i18n, fall back to raw value
  const translatedOptions = issue.options.map(opt => ({
    key: opt,
    label: t(`system_issue.option.${opt}`, { defaultValue: opt }),
  }));

  const isOtherSelected = selectedOption === 'other';
  const canSubmit = selectedOption && (!isOtherSelected || customInput.trim());

  const handleSubmit = async () => {
    if (!canSubmit) return;
    setIsSubmitting(true);
    try {
      const response = isOtherSelected ? customInput.trim() : selectedOption!;
      await onResolve({ id: issue.id, response });
      setSelectedOption(null);
      setCustomInput('');
      toast.success(t('system_issue.resolve_success'));
    } catch (err) {
      console.error('[SystemIssueDialog] resolve failed:', err);
      toast.error(t('system_issue.resolve_failed'));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDismiss = () => {
    if (issue.blocking) return; // Cannot dismiss blocking issues
    // For non-blocking, we still need to resolve — auto-resolve with empty response
    handleAutoResolve();
  };

  const handleAutoResolve = async () => {
    setIsSubmitting(true);
    try {
      await onResolve({ id: issue.id, response: '_dismissed' });
    } catch {
      // Ignore dismiss errors
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      {/* Backdrop click — only for non-blocking */}
      {!issue.blocking && (
        <div className="absolute inset-0" onClick={handleDismiss} />
      )}

      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden">
        {/* Close button — only for non-blocking */}
        {!issue.blocking && (
          <button
            onClick={handleDismiss}
            className="absolute top-4 right-4 p-1 rounded-lg text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
          >
            <X size={20} />
          </button>
        )}

        {/* Header */}
        <div className="px-6 pt-6 pb-3">
          <h2 className="text-xl font-bold text-gray-900 pr-8">{title}</h2>
          {description && (
            <p className="mt-2 text-sm text-gray-600 leading-relaxed">{description}</p>
          )}
        </div>

        {/* Options */}
        <div className="px-6 space-y-2">
          {translatedOptions.map(opt => (
            <button
              key={opt.key}
              onClick={() => {
                setSelectedOption(opt.key);
                if (opt.key !== 'other') setCustomInput('');
              }}
              className={`w-full text-left px-4 py-3 rounded-lg border-2 transition-colors ${
                selectedOption === opt.key
                  ? 'border-blue-500 bg-blue-50 text-blue-700 font-medium'
                  : 'border-gray-200 hover:border-gray-300 text-gray-700'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>

        {/* Custom input when "other" is selected */}
        {isOtherSelected && (
          <div className="px-6 mt-3">
            <input
              type="text"
              value={customInput}
              onChange={(e) => setCustomInput(e.target.value)}
              placeholder={t('system_issue.input_placeholder')}
              className="w-full px-4 py-3 border-2 border-gray-200 rounded-lg focus:border-blue-500 focus:outline-none transition-colors"
              autoFocus
            />
          </div>
        )}

        {/* Submit button */}
        <div className="px-6 py-5">
          <button
            onClick={handleSubmit}
            disabled={!canSubmit || isSubmitting}
            className={`w-full py-3 rounded-lg font-medium transition-colors ${
              canSubmit && !isSubmitting
                ? 'bg-blue-600 text-white hover:bg-blue-700 active:bg-blue-800'
                : 'bg-gray-200 text-gray-400 cursor-not-allowed'
            }`}
          >
            {isSubmitting ? '...' : t('common.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
