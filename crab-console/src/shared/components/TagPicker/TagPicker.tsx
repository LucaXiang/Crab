import React, { useMemo, useState } from 'react';
import { Search } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StoreTag } from '@/core/types/store';

interface TagPickerProps {
  tags: StoreTag[];
  selectedIds: number[];
  onToggle: (id: number) => void;
  themeColor?: string;
  emptyText?: string;
}

const FALLBACK_COLOR = '#6366f1';
const SEARCH_THRESHOLD = 8;

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!m) return null;
  return { r: parseInt(m[1], 16), g: parseInt(m[2], 16), b: parseInt(m[3], 16) };
}

export const TagPicker: React.FC<TagPickerProps> = ({
  tags,
  selectedIds,
  onToggle,
  emptyText,
}) => {
  const { t } = useI18n();
  const [search, setSearch] = useState('');
  const activeTags = useMemo(() => tags.filter(tag => tag.is_active), [tags]);

  const filtered = useMemo(() => {
    if (!search.trim()) return activeTags;
    const q = search.toLowerCase();
    return activeTags.filter(tag => tag.name.toLowerCase().includes(q));
  }, [activeTags, search]);

  if (activeTags.length === 0) {
    return <span className="text-xs text-gray-400">{emptyText ?? t('settings.attribute.no_options')}</span>;
  }

  const selectedCount = activeTags.filter(tag => selectedIds.includes(tag.source_id)).length;

  return (
    <div className="space-y-2">
      {activeTags.length >= SEARCH_THRESHOLD && (
        <div className="relative">
          <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('common.hint.search_placeholder')}
            className="w-full pl-8 pr-3 py-1.5 text-xs border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
          />
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        {filtered.map((tag) => {
          const isSelected = selectedIds.includes(tag.source_id);
          const color = tag.color || FALLBACK_COLOR;
          const rgb = hexToRgb(color);
          return (
            <button
              key={tag.source_id}
              type="button"
              onClick={() => onToggle(tag.source_id)}
              className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors ${
                isSelected
                  ? 'text-white border-transparent'
                  : 'bg-white text-gray-600 border-gray-200 hover:border-gray-300'
              }`}
              style={isSelected ? {
                backgroundColor: color,
                borderColor: color,
              } : rgb ? {
                borderColor: `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.3)`,
              } : undefined}
            >
              {!isSelected && (
                <span
                  className="inline-block w-2 h-2 rounded-full mr-1.5"
                  style={{ backgroundColor: color }}
                />
              )}
              {tag.name}
            </button>
          );
        })}
      </div>
      {selectedCount > 0 && (
        <p className="text-xs text-gray-400">{selectedCount} {t('common.selection.selected')}</p>
      )}
    </div>
  );
};
