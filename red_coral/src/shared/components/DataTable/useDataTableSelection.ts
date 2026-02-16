import React from 'react';

interface UseDataTableSelectionParams<T> {
  data: T[];
  currentData: T[];
  getRowKey: (item: T) => string | number;
  onBatchDelete?: (items: T[]) => void;
  propIsSelectionMode?: boolean;
  onSelectionModeChange?: (mode: boolean) => void;
}

interface UseDataTableSelectionReturn {
  selectedKeys: Set<string | number>;
  isSelectionMode: boolean;
  setIsSelectionMode: (mode: boolean) => void;
  toggleSelection: (key: string | number) => void;
  toggleSelectAll: () => void;
  handleBatchDelete: () => void;
  exitSelectionMode: () => void;
}

export function useDataTableSelection<T>({
  data,
  currentData,
  getRowKey,
  onBatchDelete,
  propIsSelectionMode,
  onSelectionModeChange,
}: UseDataTableSelectionParams<T>): UseDataTableSelectionReturn {
  const [selectedKeys, setSelectedKeys] = React.useState<Set<string | number>>(new Set());
  const [internalSelectionMode, setInternalSelectionMode] = React.useState(false);

  const isSelectionMode = propIsSelectionMode ?? internalSelectionMode;

  const setIsSelectionMode = (mode: boolean) => {
    if (onSelectionModeChange) {
      onSelectionModeChange(mode);
    } else {
      setInternalSelectionMode(mode);
    }
  };

  // Exit selection mode and clear selection when data changes
  React.useEffect(() => {
    setSelectedKeys(new Set());
    setIsSelectionMode(false);
  }, [data]);

  const toggleSelection = (key: string | number) => {
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

  return {
    selectedKeys,
    isSelectionMode,
    setIsSelectionMode,
    toggleSelection,
    toggleSelectAll,
    handleBatchDelete,
    exitSelectionMode,
  };
}
