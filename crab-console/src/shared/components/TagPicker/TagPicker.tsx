import React from 'react';
import { useI18n } from '@/hooks/useI18n';
import type { StoreTag } from '@/core/types/store';

const COLOR_MAP: Record<string, { active: string; inactive: string }> = {
  blue:   { active: 'bg-blue-50 text-blue-700 border-blue-300',     inactive: 'bg-white text-gray-600 border-gray-200 hover:border-gray-300' },
  purple: { active: 'bg-purple-50 text-purple-700 border-purple-300', inactive: 'bg-white text-gray-600 border-gray-200 hover:border-gray-300' },
  teal:   { active: 'bg-teal-50 text-teal-700 border-teal-300',     inactive: 'bg-white text-gray-600 border-gray-200 hover:border-gray-300' },
  orange: { active: 'bg-orange-50 text-orange-700 border-orange-300', inactive: 'bg-white text-gray-600 border-gray-200 hover:border-gray-300' },
};

interface TagPickerProps {
  tags: StoreTag[];
  selectedIds: number[];
  onToggle: (id: number) => void;
  themeColor?: string;
  emptyText?: string;
}

export const TagPicker: React.FC<TagPickerProps> = ({
  tags,
  selectedIds,
  onToggle,
  themeColor = 'blue',
  emptyText,
}) => {
  const { t } = useI18n();
  const activeTags = tags.filter(tag => tag.is_active);
  const colors = COLOR_MAP[themeColor] ?? COLOR_MAP.blue;

  if (activeTags.length === 0) {
    return <span className="text-xs text-gray-400">{emptyText ?? t('settings.attribute.no_options')}</span>;
  }

  return (
    <div className="flex flex-wrap gap-2">
      {activeTags.map((tag) => (
        <button
          key={tag.source_id}
          type="button"
          onClick={() => onToggle(tag.source_id)}
          className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors ${
            selectedIds.includes(tag.source_id) ? colors.active : colors.inactive
          }`}
        >
          {tag.name}
        </button>
      ))}
    </div>
  );
};
