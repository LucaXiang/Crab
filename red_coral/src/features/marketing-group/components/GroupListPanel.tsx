import React from 'react';
import type { MarketingGroup } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';

interface GroupListPanelProps {
  groups: MarketingGroup[];
  selectedGroupId: number | null;
  onSelectGroup: (id: number) => void;
  searchQuery: string;
}

export const GroupListPanel: React.FC<GroupListPanelProps> = ({
  groups,
  selectedGroupId,
  onSelectGroup,
  searchQuery,
}) => {
  const { t } = useI18n();

  const filteredGroups = React.useMemo(() => {
    if (!searchQuery.trim()) return groups;
    const q = searchQuery.toLowerCase();
    return groups.filter(
      (g) => g.display_name.toLowerCase().includes(q) || g.name.toLowerCase().includes(q)
    );
  }, [groups, searchQuery]);

  return (
    <div className="w-80 shrink-0 flex flex-col h-full overflow-hidden bg-gray-50">
      {/* Header */}
      <div className="px-4 py-3 border-b border-gray-200">
        <span className="text-sm font-medium text-gray-700">
          {t('settings.marketing_group.list_title')} ({filteredGroups.length})
        </span>
      </div>

      {/* Group list */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {filteredGroups.length === 0 ? (
          <div className="text-center py-8 text-gray-400 text-sm">
            {searchQuery ? t('common.empty.no_results') : t('settings.marketing_group.empty')}
          </div>
        ) : (
          filteredGroups.map((group) => {
            const isSelected = group.id === selectedGroupId;
            return (
              <button
                key={group.id}
                onClick={() => onSelectGroup(group.id)}
                className={`
                  w-full text-left p-3 rounded-xl transition-all duration-150
                  ${isSelected
                    ? 'ring-2 ring-violet-400 bg-violet-50 shadow-md'
                    : `bg-white hover:bg-gray-50 shadow-sm hover:shadow ${!group.is_active ? 'opacity-50' : ''}`
                  }
                `}
              >
                {/* Row 1: Status dot + Name */}
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className={`w-2 h-2 rounded-full shrink-0 ${
                      !group.is_active ? 'bg-gray-400' : 'bg-violet-500'
                    }`}
                  />
                  <span className="font-medium text-gray-900 truncate">
                    {group.display_name}
                  </span>
                </div>

                {/* Row 2: Internal name */}
                <div className="text-xs text-gray-400 mt-1 ml-4">{group.name}</div>

                {/* Row 3: Description preview */}
                {group.description && (
                  <div className="text-xs text-gray-400 mt-0.5 ml-4 truncate">
                    {group.description}
                  </div>
                )}
              </button>
            );
          })
        )}
      </div>
    </div>
  );
};
