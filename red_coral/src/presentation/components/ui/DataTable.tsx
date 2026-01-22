import React from 'react';
import { Edit3, Trash2, ChevronLeft, ChevronRight, Check, X, ListChecks } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

export interface Column<T> {
  key: string;
  header: string;
  width?: string;
  render?: (item: T) => React.ReactNode;
  align?: 'left' | 'center' | 'right';
}

interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  loading?: boolean;
  emptyText?: string;
  onEdit?: (item: T) => void;
  onDelete?: (item: T) => void;
  isEditable?: (item: T) => boolean;
  isDeletable?: (item: T) => boolean;
  onBatchDelete?: (items: T[]) => void;
  getRowKey: (item: T) => string;
  pageSize?: number;
  selectable?: boolean;
  themeColor?: 'blue' | 'orange' | 'purple' | 'teal' | 'indigo';
  // Server-side pagination props
  totalItems?: number;
  currentPage?: number;
  onPageChange?: (page: number) => void;
}

const THEMES = {
  blue: {
    headerBg: 'bg-blue-50',
    headerBorder: 'border-blue-100',
    headerText: 'text-blue-700',
    rowHover: 'hover:bg-blue-50/50',
    rowSelected: 'bg-blue-50/70',
    checkboxActive: 'bg-blue-600 border-blue-600',
    checkboxHover: 'hover:border-blue-500',
    paginationActive: 'bg-blue-600 text-white',
    selectBtn: 'text-blue-600 bg-blue-50 hover:bg-blue-100 border-blue-100',
    selectModeHeader: 'bg-blue-50 border-blue-100',
    selectModeText: 'text-blue-700',
  },
  orange: {
    headerBg: 'bg-orange-50',
    headerBorder: 'border-orange-100',
    headerText: 'text-orange-700',
    rowHover: 'hover:bg-orange-50/50',
    rowSelected: 'bg-orange-50/70',
    checkboxActive: 'bg-orange-600 border-orange-600',
    checkboxHover: 'hover:border-orange-500',
    paginationActive: 'bg-orange-600 text-white',
    selectBtn: 'text-orange-600 bg-orange-50 hover:bg-orange-100 border-orange-100',
    selectModeHeader: 'bg-orange-50 border-orange-100',
    selectModeText: 'text-orange-700',
  },
  purple: {
    headerBg: 'bg-purple-50',
    headerBorder: 'border-purple-100',
    headerText: 'text-purple-700',
    rowHover: 'hover:bg-purple-50/50',
    rowSelected: 'bg-purple-50/70',
    checkboxActive: 'bg-purple-600 border-purple-600',
    checkboxHover: 'hover:border-purple-500',
    paginationActive: 'bg-purple-600 text-white',
    selectBtn: 'text-purple-600 bg-purple-50 hover:bg-purple-100 border-purple-100',
    selectModeHeader: 'bg-purple-50 border-purple-100',
    selectModeText: 'text-purple-700',
  },
  teal: {
    headerBg: 'bg-teal-50',
    headerBorder: 'border-teal-100',
    headerText: 'text-teal-700',
    rowHover: 'hover:bg-teal-50/50',
    rowSelected: 'bg-teal-50/70',
    checkboxActive: 'bg-teal-600 border-teal-600',
    checkboxHover: 'hover:border-teal-500',
    paginationActive: 'bg-teal-600 text-white',
    selectBtn: 'text-teal-600 bg-teal-50 hover:bg-teal-100 border-teal-100',
    selectModeHeader: 'bg-teal-50 border-teal-100',
    selectModeText: 'text-teal-700',
  },
  indigo: {
    headerBg: 'bg-indigo-50',
    headerBorder: 'border-indigo-100',
    headerText: 'text-indigo-700',
    rowHover: 'hover:bg-indigo-50/50',
    rowSelected: 'bg-indigo-50/70',
    checkboxActive: 'bg-indigo-600 border-indigo-600',
    checkboxHover: 'hover:border-indigo-500',
    paginationActive: 'bg-indigo-600 text-white',
    selectBtn: 'text-indigo-600 bg-indigo-50 hover:bg-indigo-100 border-indigo-100',
    selectModeHeader: 'bg-indigo-50 border-indigo-100',
    selectModeText: 'text-indigo-700',
  },
};

export function DataTable<T>({
  data,
  columns,
  loading = false,
  emptyText,
  onEdit,
  onDelete,
  isEditable,
  isDeletable,
  onBatchDelete,
  getRowKey,
  pageSize = 10,
  selectable = true,
  themeColor = 'blue',
  totalItems,
  currentPage: propCurrentPage,
  onPageChange,
}: DataTableProps<T>) {
  const { t } = useI18n();
  const theme = THEMES[themeColor] || THEMES.blue;
  const [internalPage, setInternalPage] = React.useState(1);
  const [selectedKeys, setSelectedKeys] = React.useState<Set<string>>(new Set());
  const [isSelectionMode, setIsSelectionMode] = React.useState(false);
  const [longPressTimer, setLongPressTimer] = React.useState<NodeJS.Timeout | null>(null);

  const isServerSide = typeof totalItems === 'number';
  const currentPage = isServerSide ? (propCurrentPage || 1) : internalPage;
  const finalTotalItems = isServerSide ? (totalItems || 0) : data.length;
  const totalPages = Math.ceil(finalTotalItems / pageSize);

  const currentData = isServerSide ? data : data.slice((currentPage - 1) * pageSize, currentPage * pageSize);

  const startIndex = (currentPage - 1) * pageSize;
  const endIndex = startIndex + currentData.length;

  // Reset to page 1 when data changes (only for client-side)
  React.useEffect(() => {
    if (!isServerSide) {
      setInternalPage(1);
    }
  }, [data.length, isServerSide]);

  // Exit selection mode and clear selection when data changes
  React.useEffect(() => {
    setSelectedKeys(new Set());
    setIsSelectionMode(false);
  }, [data]);

  // Cleanup long press timer on unmount
  React.useEffect(() => {
    return () => {
      if (longPressTimer) {
        clearTimeout(longPressTimer);
      }
    };
  }, [longPressTimer]);

  const showActions = onEdit || onDelete;
  const canBatchDelete = selectable && onBatchDelete;

  const getAlignClass = (align?: 'left' | 'center' | 'right') => {
    switch (align) {
      case 'center': return 'text-center';
      case 'right': return 'text-right';
      default: return 'text-left';
    }
  };

  const toggleSelection = (key: string) => {
    const newSet = new Set(selectedKeys);
    if (newSet.has(key)) {
      newSet.delete(key);
    } else {
      newSet.add(key);
    }
    setSelectedKeys(newSet);
  };

  const toggleSelectAll = () => {
    if (selectedKeys.size === currentData.length) {
      setSelectedKeys(new Set());
    } else {
      setSelectedKeys(new Set(currentData.map(getRowKey)));
    }
  };

  const handleBatchDelete = () => {
    if (onBatchDelete && selectedKeys.size > 0) {
      const selectedItems = data.filter((item) => selectedKeys.has(getRowKey(item)));
      onBatchDelete(selectedItems);
    }
  };

  const exitSelectionMode = () => {
    setIsSelectionMode(false);
    setSelectedKeys(new Set());
  };

  const handlePageChange = (page: number) => {
    const p = Math.max(1, Math.min(totalPages, page));
    if (isServerSide && onPageChange) {
      onPageChange(p);
    } else {
      setInternalPage(p);
    }
  };

  // Long press handler
  const handleLongPress = (item: T, callback?: (item: T) => void) => {
    if (!callback) return;
    const timer = setTimeout(() => {
      callback(item);
    }, 500); // 500ms long press
    setLongPressTimer(timer);
  };

  const clearLongPress = () => {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      setLongPressTimer(null);
    }
  };

  // Double click handler
  const [lastClickTime, setLastClickTime] = React.useState(0);
  const handleDoubleClick = (item: T, callback?: (item: T) => void) => {
    if (!callback) return;
    const now = Date.now();
    if (now - lastClickTime < 300) {
      // Double click detected
      callback(item);
      setLastClickTime(0);
    } else {
      setLastClickTime(now);
    }
  };

  const getPaginationRange = () => {
    const siblingCount = 1;
    // Total numbers to show: first + last + current + 2*siblings + 2*dots = 7 (approx)
    if (totalPages <= 7) {
      return Array.from({ length: totalPages }, (_, i) => i + 1);
    }

    const leftSiblingIndex = Math.max(currentPage - siblingCount, 1);
    const rightSiblingIndex = Math.min(currentPage + siblingCount, totalPages);

    const showLeftDots = leftSiblingIndex > 2;
    const showRightDots = rightSiblingIndex < totalPages - 1;

    if (!showLeftDots && showRightDots) {
      const leftItemCount = 5;
      const leftRange = Array.from({ length: leftItemCount }, (_, i) => i + 1);
      return [...leftRange, '...', totalPages];
    }

    if (showLeftDots && !showRightDots) {
      const rightItemCount = 5;
      const rightRange = Array.from({ length: rightItemCount }, (_, i) => totalPages - rightItemCount + i + 1);
      return [1, '...', ...rightRange];
    }

    if (showLeftDots && showRightDots) {
      const middleRange = Array.from({ length: rightSiblingIndex - leftSiblingIndex + 1 }, (_, i) => leftSiblingIndex + i);
      return [1, '...', ...middleRange, '...', totalPages];
    }

    return [];
  };

  const paginationRange = getPaginationRange();

  if (loading && (!data || data.length === 0)) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <div className="animate-pulse">
          <div className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200 p-4">
            <div className="flex gap-4">
              {columns.map((_, i) => (
                <div key={i} className="h-4 bg-gray-200 rounded flex-1" />
              ))}
            </div>
          </div>
          {[1, 2, 3].map((i) => (
            <div key={i} className="border-b border-gray-100 p-4">
              <div className="flex gap-4">
                {columns.map((_, j) => (
                  <div key={j} className="h-4 bg-gray-100 rounded flex-1" />
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (!loading && data.length === 0 && !isServerSide) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
        <div className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
          <div className="flex">
            {columns.map((col) => (
              <div
                key={col.key}
                className={`px-4 py-3 text-xs font-semibold text-gray-600 uppercase tracking-wider ${getAlignClass(col.align)}`}
                style={{ width: col.width, flex: col.width ? undefined : 1 }}
              >
                {col.header}
              </div>
            ))}
            {showActions && (
              <div className="px-4 py-3 text-xs font-semibold text-gray-600 uppercase tracking-wider text-right w-28">
                {t('settings.common.actions')}
              </div>
            )}
          </div>
        </div>
        <div className="flex flex-col items-center justify-center py-12 px-4">
          <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
            <svg className="w-8 h-8 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
            </svg>
          </div>
          <p className="text-gray-500 text-sm">{emptyText || t('common.label.none')}</p>
        </div>
      </div>
    );
  }

  const allSelected = currentData.length > 0 && selectedKeys.size === currentData.length;
  const someSelected = selectedKeys.size > 0 && selectedKeys.size < currentData.length;

  return (
    <div className="relative bg-white rounded-xl border border-gray-200 overflow-hidden shadow-sm">
      {/* Loading Overlay */}
      {loading && data.length > 0 && (
        <div className="absolute inset-0 bg-white/60 z-10 flex items-center justify-center backdrop-blur-[1px]">
          <div className={`w-8 h-8 border-4 border-gray-200 rounded-full animate-spin ${
            themeColor === 'orange' ? 'border-t-orange-500' :
            themeColor === 'purple' ? 'border-t-purple-600' :
            themeColor === 'teal' ? 'border-t-teal-600' :
            themeColor === 'indigo' ? 'border-t-indigo-600' :
            'border-t-blue-600'
          }`} />
        </div>
      )}

      {/* Selection Mode Header */}
      {isSelectionMode && (
        <div className={`${theme.selectModeHeader} border-b px-4 py-3 flex items-center justify-between`}>
          <div className="flex items-center gap-3">
            <span className={`text-sm ${theme.selectModeText} font-medium`}>
              {t('common.selection.selected')} {selectedKeys.size} {t('common.label.items')}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleBatchDelete}
              disabled={selectedKeys.size === 0}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-red-600 text-white rounded-lg text-xs font-medium hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Trash2 size={14} />
              <span>{t('common.action.batch_delete')}</span>
            </button>
            <button
              onClick={exitSelectionMode}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-gray-200 text-gray-700 rounded-lg text-xs font-medium hover:bg-gray-300 transition-colors"
            >
              <X size={14} />
              <span>{t('common.action.cancel')}</span>
            </button>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
        <div className="flex">
          {isSelectionMode && (
            <div className="px-4 py-3.5 flex items-center w-12">
              <button
                onClick={toggleSelectAll}
                className={`w-5 h-5 rounded border-2 flex items-center justify-center transition-colors ${
                  allSelected
                    ? theme.checkboxActive
                    : someSelected
                      ? theme.checkboxActive
                      : `border-gray-300 ${theme.checkboxHover}`
                }`}
              >
                {allSelected && <Check size={12} className="text-white" />}
                {someSelected && <div className="w-2 h-0.5 bg-white rounded" />}
              </button>
            </div>
          )}
          {columns.map((col) => (
            <div
              key={col.key}
              className={`px-4 py-3.5 text-xs font-semibold text-gray-600 uppercase tracking-wider ${getAlignClass(col.align)}`}
              style={{ width: col.width, flex: col.width ? undefined : 1 }}
            >
              {col.header}
            </div>
          ))}
          {showActions && (
            <div className="px-4 py-2 text-xs font-semibold text-gray-600 uppercase tracking-wider text-right w-32 flex items-center justify-end gap-2">
              {canBatchDelete && !isSelectionMode && (
                <button
                  onClick={() => setIsSelectionMode(true)}
                  className={`flex items-center gap-1 px-2 py-1 rounded-md transition-colors border border-transparent ${theme.selectBtn}`}
                  title={t('common.selection.select_mode')}
                >
                  <ListChecks size={14} />
                  <span className="hidden sm:inline scale-90 origin-left">{t('common.action.select')}</span>
                </button>
              )}
              <span className="py-1.5">{t('settings.common.actions')}</span>
            </div>
          )}
        </div>
        {/* Hint for interactive actions */}
        {onEdit && (
          <div className="px-4 pb-2 text-[10px] text-gray-400 text-center">
            💡 {t('common.hint.long_press_to_edit')}
          </div>
        )}
      </div>

      {/* Body */}
      <div className="divide-y divide-gray-100">
        {currentData.map((item, index) => {
          const key = getRowKey(item);
          const isSelected = selectedKeys.has(key);
          return (
            <div
              key={key}
              className={`flex items-center transition-colors ${
                isSelected
                  ? theme.rowSelected
                  : index % 2 === 0
                    ? `bg-white ${theme.rowHover}`
                    : `bg-gray-50/30 ${theme.rowHover}`
              }`}
              onMouseDown={() => handleLongPress(item, onEdit)}
              onMouseUp={clearLongPress}
              onMouseLeave={clearLongPress}
              onTouchStart={() => handleLongPress(item, onEdit)}
              onTouchEnd={clearLongPress}
              onDoubleClick={() => handleDoubleClick(item, onEdit)}
            >
              {isSelectionMode && (
                <div className="px-4 py-3.5 flex items-center w-12">
                  <button
                    onClick={() => toggleSelection(key)}
                    className={`w-5 h-5 rounded border-2 flex items-center justify-center transition-colors ${
                      isSelected
                        ? theme.checkboxActive
                        : `border-gray-300 ${theme.checkboxHover}`
                    }`}
                  >
                    {isSelected && <Check size={12} className="text-white" />}
                  </button>
                </div>
              )}
              {columns.map((col) => (
                <div
                  key={col.key}
                  className={`px-4 py-3.5 text-sm text-gray-700 ${getAlignClass(col.align)}`}
                  style={{ width: col.width, flex: col.width ? undefined : 1 }}
                >
                  {col.render ? col.render(item) : (item as any)[col.key]}
                </div>
              ))}
              {showActions && (
                <div className="px-4 py-2 flex items-center justify-end gap-2 w-32">
                  {!isSelectionMode && (
                    <>
                      {onEdit && (!isEditable || isEditable(item)) && (
                        <button
                          onClick={() => onEdit(item)}
                          className="p-2 bg-amber-50 text-amber-700 rounded-lg hover:bg-amber-100 transition-colors border border-amber-200/50"
                          title={t('common.action.edit')}
                        >
                          <Edit3 size={14} />
                        </button>
                      )}
                      {onDelete && (!isDeletable || isDeletable(item)) && (
                        <button
                          onClick={() => onDelete(item)}
                          className="p-2 bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors border border-red-200/50"
                          title={t('common.action.delete')}
                        >
                          <Trash2 size={14} />
                        </button>
                      )}
                    </>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Footer */}
      <div className="bg-gray-50 border-t border-gray-200 px-4 py-3 flex items-center justify-between">
        <div className="flex items-center gap-3">
          {totalPages > 1 && (
            <div className="text-xs text-gray-500">
              {t('common.selection.showing')} {startIndex + 1}-{Math.min(endIndex, finalTotalItems)} / {finalTotalItems}
            </div>
          )}
          {totalPages <= 1 && (
            <div className="text-xs text-gray-500">
              {t('common.label.total')} {finalTotalItems} {t('common.label.items')}
            </div>
          )}
        </div>
        {totalPages > 0 && (
          <div className="flex items-center gap-1">
            <button
              onClick={() => handlePageChange(currentPage - 1)}
              disabled={currentPage === 1}
              className="p-1.5 rounded-lg hover:bg-gray-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronLeft size={16} className="text-gray-600" />
            </button>
            {paginationRange.map((page, index) => {
              if (page === '...') {
                return (
                  <span key={`dots-${index}`} className="px-2 text-gray-400 select-none flex items-center">
                    ...
                  </span>
                );
              }
              return (
                <button
                  key={page}
                  onClick={() => handlePageChange(page as number)}
                  className={`min-w-[28px] h-7 px-2 rounded-lg text-xs font-medium transition-colors ${
                    page === currentPage
                      ? theme.paginationActive
                      : 'text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {page}
                </button>
              );
            })}
            <button
              onClick={() => handlePageChange(currentPage + 1)}
              disabled={currentPage === totalPages}
              className="p-1.5 rounded-lg hover:bg-gray-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronRight size={16} className="text-gray-600" />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
