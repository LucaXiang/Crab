import { create } from 'zustand';
import { Zone, Table } from '@/core/domain/types';
import { createTauriClient } from '@/infrastructure/api';
import { logger } from '@/utils/logger';

const api = createTauriClient();

/**
 * Cache entry with timestamp for LRU eviction
 */
interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

/**
 * Cache configuration for zones and tables
 * Longer TTL since these rarely change
 */
const CACHE_CONFIG = {
  ZONES_TTL_MS: 30 * 60 * 1000, // 30 minutes
  TABLES_TTL_MS: 30 * 60 * 1000, // 30 minutes
  ENABLE_CACHE: true,
};

/**
 * Check if cache entry is still valid
 */
function isCacheValid(entry: CacheEntry<any> | null): boolean {
  if (!entry) return false;
  return Date.now() - entry.timestamp < CACHE_CONFIG.ZONES_TTL_MS;
}

interface ZoneTableStore {
  // Cache state (private)
  _zonesCache: CacheEntry<Zone[]> | null;
  _tablesCache: Map<string, CacheEntry<Table[]>>;

  // State
  zones: Zone[];
  tables: Table[];
  isLoadingZones: boolean;
  isLoadingTables: boolean;
  error: string | null;

  // Actions
  loadZones: () => Promise<Zone[]>;
  loadTables: (zoneId?: string) => Promise<Table[]>;
  clearCache: () => void;
  invalidateZones: () => void;
  invalidateTables: (zoneId?: string) => void;
}

/**
 * Generate cache key for tables query
 */
function getTablesCacheKey(zoneId?: string): string {
  return zoneId || 'ALL';
}

export const useZoneTableStore = create<ZoneTableStore>((set, get) => ({
  // Initial State
  _zonesCache: null,
  _tablesCache: new Map(),
  zones: [],
  tables: [],
  isLoadingZones: false,
  isLoadingTables: false,
  error: null,

  loadZones: async () => {
    const { _zonesCache } = get();

    // Check cache first
    if (CACHE_CONFIG.ENABLE_CACHE && _zonesCache && isCacheValid(_zonesCache)) {
      set({ zones: _zonesCache.data, isLoadingZones: false });
      return _zonesCache.data;
    }

    set({ isLoadingZones: true, error: null });

    try {
      const zones = await api.listZones();

      // Store in cache
      if (CACHE_CONFIG.ENABLE_CACHE) {
        set({
          _zonesCache: {
            data: zones,
            timestamp: Date.now(),
          },
        });
      }

      set({ zones, isLoadingZones: false });
      return zones;
    } catch (error) {
      set({
        isLoadingZones: false,
        error: error instanceof Error ? error.message : 'Failed to load zones',
      });
      logger.error('Failed to load zones', error, { component: 'ZoneTableStore', action: 'loadZones' });
      throw error;
    }
  },

  loadTables: async (zoneId?: string) => {
    const { _tablesCache } = get();
    const cacheKey = getTablesCacheKey(zoneId);

    // Check cache first
    if (CACHE_CONFIG.ENABLE_CACHE) {
      const cached = _tablesCache.get(cacheKey);
      if (cached && Date.now() - cached.timestamp < CACHE_CONFIG.TABLES_TTL_MS) {
        set({ tables: cached.data, isLoadingTables: false });
        return cached.data;
      }
    }

    set({ isLoadingTables: true, error: null });

    try {
      const tables = await api.listTables();

      // Store in cache
      if (CACHE_CONFIG.ENABLE_CACHE) {
        const newCache = new Map(_tablesCache);
        newCache.set(cacheKey, {
          data: tables,
          timestamp: Date.now(),
        });
        set({ _tablesCache: newCache });
      }

      set({ tables, isLoadingTables: false });
      return tables;
    } catch (error) {
      set({
        isLoadingTables: false,
        error: error instanceof Error ? error.message : 'Failed to load tables',
      });
      logger.error('Failed to load tables', error, { component: 'ZoneTableStore', action: 'loadTables' });
      throw error;
    }
  },

  clearCache: () => {
    set({
      _zonesCache: null,
      _tablesCache: new Map(),
    });
  },

  invalidateZones: () => {
    set({ _zonesCache: null });
  },

  invalidateTables: (zoneId?: string) => {
    const { _tablesCache } = get();
    if (zoneId) {
      const cacheKey = getTablesCacheKey(zoneId);
      const newCache = new Map(_tablesCache);
      newCache.delete(cacheKey);
      set({ _tablesCache: newCache });
    } else {
      // Invalidate all table caches
      set({ _tablesCache: new Map() });
    }
  },
}));

// Convenience hooks
// Convenience hooks removed; use useZoneTableStore directly where needed
