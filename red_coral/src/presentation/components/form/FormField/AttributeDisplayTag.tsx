import React, { useMemo } from 'react';
import { Lock, X } from 'lucide-react';
import { AttributeTemplate } from '@/core/domain/types';
import { useAttributeStore } from '@/core/stores/product/useAttributeStore';

export interface AttributeDisplayTagProps {
  attribute: AttributeTemplate;
  defaultOptionIds?: (string | number)[];
  onRemove?: (attrId: string | number) => void;
  isInherited?: boolean;
  t: (key: string) => string;
}

/**
 * Displays an attribute as a styled tag with optional default options
 * Shows lock icon for inherited attributes and X button for removable ones
 */
export const AttributeDisplayTag: React.FC<AttributeDisplayTagProps> = ({
  attribute,
  defaultOptionIds = [],
  onRemove,
  isInherited,
  t
}) => {
  const { getOptionsByAttributeId } = useAttributeStore();

  const defaultOptions = useMemo(() => {
    if (defaultOptionIds.length === 0) return [];

    const allOptions = getOptionsByAttributeId(attribute.id);
    return defaultOptionIds
      .map((id) => allOptions.find(o => o.id === Number(id)))
      .filter(Boolean);
  }, [attribute.id, defaultOptionIds, getOptionsByAttributeId]);

  return (
    <div className="flex items-center gap-1.5 px-3 py-1.5 bg-teal-50 text-teal-700 rounded-lg text-sm border border-teal-100">
      <span className="font-medium">{attribute.name}</span>

      {isInherited && (
        <Lock size={12} className="opacity-50" />
      )}

      {defaultOptions.length > 0 && (
        <span className="flex items-center gap-1 text-xs bg-teal-100 px-1.5 py-0.5 rounded text-teal-800 border border-teal-200">
          <span className="opacity-60 text-[10px] uppercase tracking-wider">
            {t('common.default')}:
          </span>
          <span className="font-semibold">
            {defaultOptions.map(o => o?.name).join(', ')}
          </span>
        </span>
      )}

      {!isInherited && onRemove && (
        <button
          type="button"
          onClick={() => onRemove(String(attribute.id))}
          className="ml-auto hover:bg-teal-200/50 rounded p-0.5 transition-colors"
          title={t('common.remove')}
        >
          <X size={14} />
        </button>
      )}
    </div>
  );
};
