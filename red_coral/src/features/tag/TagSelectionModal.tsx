import React from 'react';
import { X } from 'lucide-react';
import { useTags } from './store';

interface TagSelectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  selectedTagIds: number[];
  onChange: (tagIds: number[]) => void;
  t: (key: string) => string;
}

export const TagSelectionModal: React.FC<TagSelectionModalProps> = ({
  isOpen,
  onClose,
  selectedTagIds,
  onChange,
  t,
}) => {
  // Hook must be called unconditionally before any early return
  const allTags = useTags();

  if (!isOpen) return null;

  // Filter: only show non-system tags (system tags are managed by the system)
  const selectableTags = allTags
    .filter((tag) => !tag.is_system)
    .sort((a, b) => a.display_order - b.display_order);

  const handleTagToggle = (tagId: number) => {
    const newTagIds = selectedTagIds.includes(tagId)
      ? selectedTagIds.filter((id) => id !== tagId)
      : [...selectedTagIds, tagId];
    onChange(newTagIds);
  };

  return (
    <div
      className="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-md flex flex-col max-h-[85vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 bg-gray-50/50 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">
            {t('settings.tag.select_title')}
            {selectedTagIds.length > 0 && (
              <span className="ml-2 text-sm font-normal text-gray-500">
                ({selectedTagIds.length})
              </span>
            )}
          </h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors"
          >
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto min-h-0 flex-1">
          {selectableTags.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {selectableTags.map((tag) => {
                const isSelected = selectedTagIds.includes(tag.id);
                return (
                  <button
                    key={tag.id}
                    type="button"
                    onClick={() => handleTagToggle(tag.id)}
                    className={`px-3 py-1.5 rounded-full text-sm font-medium transition-all ${
                      isSelected
                        ? 'text-white shadow-sm'
                        : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                    }`}
                    style={isSelected ? { backgroundColor: tag.color || '#14b8a6' } : undefined}
                  >
                    {tag.name}
                  </button>
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-gray-400 bg-gray-50 rounded-lg border border-dashed border-gray-200">
              <p className="text-sm">{t('settings.tag.no_tags_available')}</p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end shrink-0">
          <button
            onClick={onClose}
            className="px-6 py-2 bg-teal-600 text-white rounded-xl text-sm font-bold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20"
          >
            {t('common.action.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
