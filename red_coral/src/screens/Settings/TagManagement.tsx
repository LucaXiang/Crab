import React, { useEffect, useMemo, useState } from 'react';
import { Tags, ArrowUp, ArrowDown } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useSettingsModal, useDataVersion } from '@/core/stores/settings/useSettingsStore';
import { useTagStore } from '@/core/stores/resources';
import { createTauriClient } from '@/infrastructure/api';

const api = createTauriClient();
import { DataTable, Column } from '@/presentation/components/ui/DataTable';
import { toast } from '@/presentation/components/Toast';
import { Permission, Tag } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';

// Extracted components
import { ManagementHeader, FilterBar } from './components';

interface TagItem extends Tag {
  originalIndex: number;
}

export const TagManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission check
  const { hasPermission } = usePermission();
  const canManageTags = hasPermission(Permission.MANAGE_CATEGORIES); // Using same permission as categories

  // Use resources store for data
  const tagStore = useTagStore();
  const storeTags = tagStore.items;
  const loading = tagStore.isLoading;

  const { openModal } = useSettingsModal();
  const dataVersion = useDataVersion();

  // Local state for ordered tags (for reordering)
  const [tags, setTags] = useState<Tag[]>([]);
  const [searchQuery, setSearchQuery] = useState('');

  // Load data on mount and when dataVersion changes
  useEffect(() => {
    tagStore.fetchAll();
  }, [dataVersion]);

  // Sync local state with store
  useEffect(() => {
    setTags(storeTags);
  }, [storeTags]);

  const tagItems: TagItem[] = useMemo(
    () => tags.map((tag, index) => ({ ...tag, originalIndex: index })),
    [tags]
  );

  const filteredItems = useMemo(() => {
    if (!searchQuery.trim()) return tagItems;
    const q = searchQuery.toLowerCase();
    return tagItems.filter((tag) => tag.name.toLowerCase().includes(q));
  }, [tagItems, searchQuery]);

  const moveTag = async (index: number, direction: 'up' | 'down') => {
    if (searchQuery) return;

    const newTags = [...tags];
    const targetIndex = direction === 'up' ? index - 1 : index + 1;

    if (targetIndex < 0 || targetIndex >= newTags.length) return;

    [newTags[index], newTags[targetIndex]] = [newTags[targetIndex], newTags[index]];
    setTags(newTags);

    try {
      // Update display_order for both tags
      const updates = newTags.map((tag, idx) => ({
        id: tag.id!,
        display_order: idx
      }));

      // Update each tag's display_order
      for (const update of updates) {
        await api.updateTag(update.id, { display_order: update.display_order });
      }

      // Refresh tags from store
      await tagStore.fetchAll();
    } catch (e) {
      console.error(e);
      toast.error(t('settings.reorder_failed'));
      // Revert to server data
      await tagStore.fetchAll();
    }
  };

  const columns: Column<TagItem>[] = useMemo(
    () => [
      {
        key: 'sort',
        header: t('settings.category.header.sort'),
        width: '100px',
        align: 'center',
        render: (item) => {
          if (searchQuery) return <span className="text-gray-300">-</span>;
          const index = item.originalIndex;
          return (
            <div className="flex items-center justify-center gap-1">
              <button
                onClick={(e) => { e.stopPropagation(); moveTag(index, 'up'); }}
                disabled={!canManageTags || index === 0}
                className="p-1 hover:bg-gray-100 rounded text-gray-400 hover:text-indigo-600 disabled:opacity-30 transition-colors"
              >
                <ArrowUp size={14} />
              </button>
              <button
                onClick={(e) => { e.stopPropagation(); moveTag(index, 'down'); }}
                disabled={!canManageTags || index === tags.length - 1}
                className="p-1 hover:bg-gray-100 rounded text-gray-400 hover:text-indigo-600 disabled:opacity-30 transition-colors"
              >
                <ArrowDown size={14} />
              </button>
            </div>
          );
        },
      },
      {
        key: 'name',
        header: t('settings.tag.name'),
        render: (item) => (
          <div className="flex items-center gap-3">
            <span
              className="px-3 py-1 rounded-full text-sm font-medium text-white"
              style={{ backgroundColor: item.color || '#3B82F6' }}
            >
              {item.name}
            </span>
            {item.is_system && (
              <span className="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-500 rounded">
                {t('common.label.system')}
              </span>
            )}
          </div>
        ),
      },
      {
        key: 'color',
        header: t('settings.tag.color'),
        width: '120px',
        render: (item) => (
          <div className="flex items-center gap-2">
            <div
              className="w-6 h-6 rounded-md border border-gray-200"
              style={{ backgroundColor: item.color || '#3B82F6' }}
            />
            <span className="text-sm text-gray-600 font-mono">{item.color || '#3B82F6'}</span>
          </div>
        ),
      },
      {
        key: 'display_order',
        header: t('settings.tag.display_order'),
        width: '100px',
        align: 'center',
        render: (item) => (
          <span className="text-sm text-gray-500">{item.display_order}</span>
        ),
      },
    ],
    [t, tags, searchQuery, canManageTags]
  );

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={Tags}
        title={t('settings.tag.title')}
        description={t('settings.tag.description')}
        addButtonText={t('settings.tag.add_tag')}
        onAdd={() => openModal('TAG', 'CREATE')}
        themeColor="indigo"
        permission={Permission.MANAGE_CATEGORIES}
      />

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('common.hint.search_placeholder')}
        totalCount={filteredItems.length}
        countUnit={t('settings.tag.unit')}
        themeColor="indigo"
      />

      <DataTable
        data={filteredItems}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id || item.name}
        onEdit={canManageTags ? (item) => openModal('TAG', 'EDIT', item) : undefined}
        onDelete={canManageTags ? (item) => openModal('TAG', 'DELETE', item) : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="indigo"
      />
    </div>
  );
});
