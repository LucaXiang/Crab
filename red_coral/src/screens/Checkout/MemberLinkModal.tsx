/**
 * MemberLinkModal - 搜索并关联会员到订单
 */

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { X, Search, UserCheck, Loader2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { searchMembers } from '@/features/member/mutations';
import { linkMember } from '@/core/stores/order/commands';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import type { MemberWithGroup } from '@/core/domain/types/api';

interface MemberLinkModalProps {
  isOpen: boolean;
  orderId: string;
  onClose: () => void;
}

export const MemberLinkModal: React.FC<MemberLinkModalProps> = ({ isOpen, orderId, onClose }) => {
  const { t } = useI18n();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<MemberWithGroup[]>([]);
  const [loading, setLoading] = useState(false);
  const [linking, setLinking] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!isOpen) {
      setQuery('');
      setResults([]);
    }
  }, [isOpen]);

  const doSearch = useCallback(async (q: string) => {
    if (!q.trim()) {
      setResults([]);
      return;
    }
    setLoading(true);
    try {
      const members = await searchMembers(q.trim());
      setResults(members);
    } catch (e) {
      logger.error('Member search failed', e);
      toast.error(t('checkout.member.search_failed'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  // Auto-search on input change with debounce
  useEffect(() => {
    clearTimeout(debounceRef.current);
    if (!query.trim()) {
      setResults([]);
      return;
    }
    debounceRef.current = setTimeout(() => {
      doSearch(query);
    }, 300);
    return () => clearTimeout(debounceRef.current);
  }, [query, doSearch]);

  const handleLink = useCallback(async (member: MemberWithGroup) => {
    setLinking(true);
    try {
      await linkMember(orderId, member.id);
      toast.success(t('checkout.member.linked', { name: member.name }));
      onClose();
    } catch (e) {
      logger.error('Link member failed', e);
      toast.error(t('checkout.member.link_failed'));
    } finally {
      setLinking(false);
    }
  }, [orderId, onClose, t]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full max-h-[80vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="p-5 border-b border-gray-200 flex items-center justify-between shrink-0">
          <h3 className="text-xl font-bold text-gray-800 flex items-center gap-2">
            <UserCheck size={24} className="text-gray-600" />
            {t('checkout.member.search_title')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full transition-colors">
            <X size={20} />
          </button>
        </div>

        {/* Search */}
        <div className="p-4 border-b border-gray-100 shrink-0">
          <div className="relative">
            <Search size={18} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t('checkout.member.search_placeholder')}
              className="w-full pl-10 pr-10 py-3 border border-gray-300 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-blue-500 text-lg"
              autoFocus
            />
            {loading && (
              <Loader2 size={18} className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 animate-spin" />
            )}
          </div>
        </div>

        {/* Results */}
        <div className="flex-1 overflow-y-auto">
          {results.length === 0 ? (
            <div className="p-8 text-center text-gray-400">
              {query.trim() ? t('checkout.member.no_results') : t('checkout.member.search_hint')}
            </div>
          ) : (
            <div className="py-1">
              {results.map((member, idx) => (
                <button
                  key={member.id}
                  onClick={() => handleLink(member)}
                  disabled={linking || !member.is_active}
                  className={`w-full text-left px-4 py-3 hover:bg-gray-50 transition-colors flex items-center justify-between disabled:opacity-50 ${
                    idx === results.length - 1 ? 'rounded-b-2xl' : ''
                  }`}
                >
                  <div>
                    <div className="font-medium text-gray-800 flex items-center gap-2">
                      {member.name}
                      <span className="text-xs bg-violet-100 text-violet-700 px-2 py-0.5 rounded-full">
                        {member.marketing_group_name}
                      </span>
                    </div>
                    <div className="text-sm text-gray-500 mt-0.5 flex items-center gap-3">
                      {member.phone && <span>{member.phone}</span>}
                      {member.card_number && <span>{member.card_number}</span>}
                    </div>
                  </div>
                  <UserCheck size={20} className="text-gray-400 shrink-0" />
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
