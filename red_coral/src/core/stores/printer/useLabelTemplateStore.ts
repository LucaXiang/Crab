/**
 * Label Template Store - Fetches and manages label templates from server API
 *
 * 类型统一使用 snake_case，与 Rust 后端 serde 一致，无需映射层
 */

import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { LabelTemplate, LabelField } from '@/core/domain/types/print';
import type { LabelTemplateCreate, LabelTemplateUpdate } from '@/core/domain/types/api';
import { DEFAULT_LABEL_TEMPLATES } from '@/core/domain/types/print';

// Lazy-load API client to avoid initialization issues
const getApi = () => createTauriClient();

// 从 API 响应提取 ID (去掉 "label_template:" 前缀)
function mapApiToFrontend(apiTemplate: Record<string, unknown>): LabelTemplate {
  return {
    ...apiTemplate,
    id: (apiTemplate.id as string)?.split(':')[1] || apiTemplate.id as string,
    padding: apiTemplate.padding as number || 2,
    fields: (apiTemplate.fields as LabelField[]) || [],
    is_default: apiTemplate.is_default as boolean || false,
    is_active: apiTemplate.is_active as boolean ?? true,
    created_at: apiTemplate.created_at as number || Date.now(),
    updated_at: apiTemplate.updated_at as number || Date.now(),
    width_mm: apiTemplate.width_mm as number ?? apiTemplate.width as number,
    height_mm: apiTemplate.height_mm as number ?? apiTemplate.height as number,
  } as LabelTemplate;
}

// 构造 Create payload
function toCreatePayload(template: Partial<LabelTemplate>): LabelTemplateCreate {
  return {
    name: template.name || '',
    description: template.description,
    width: template.width_mm || template.width || 40,
    height: template.height_mm || template.height || 30,
    fields: template.fields || [],
    is_default: template.is_default || false,
    is_active: template.is_active ?? true,
    padding_mm_x: template.padding_mm_x,
    padding_mm_y: template.padding_mm_y,
    render_dpi: template.render_dpi,
    test_data: template.test_data,
  };
}

// 构造 Update payload
function toUpdatePayload(template: Partial<LabelTemplate>): LabelTemplateUpdate {
  const update: LabelTemplateUpdate = {};
  if (template.name !== undefined) update.name = template.name;
  if (template.description !== undefined) update.description = template.description;
  if (template.width !== undefined) update.width = template.width;
  if (template.height !== undefined) update.height = template.height;
  if (template.fields !== undefined) update.fields = template.fields;
  if (template.is_default !== undefined) update.is_default = template.is_default;
  if (template.is_active !== undefined) update.is_active = template.is_active;
  if (template.width_mm !== undefined) update.width = template.width_mm;
  if (template.height_mm !== undefined) update.height = template.height_mm;
  if (template.padding_mm_x !== undefined) update.padding_mm_x = template.padding_mm_x;
  if (template.padding_mm_y !== undefined) update.padding_mm_y = template.padding_mm_y;
  if (template.render_dpi !== undefined) update.render_dpi = template.render_dpi;
  if (template.test_data !== undefined) update.test_data = template.test_data;
  return update;
}

interface LabelTemplateStore {
  // State
  templates: LabelTemplate[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;
  lastVersion: number;

  // Actions
  fetchAll: (force?: boolean) => Promise<void>;
  createTemplate: (template: Partial<LabelTemplate>) => Promise<LabelTemplate>;
  updateTemplate: (id: string, template: Partial<LabelTemplate>) => Promise<LabelTemplate>;
  deleteTemplate: (id: string) => Promise<void>;
  duplicateTemplate: (template: LabelTemplate) => Promise<LabelTemplate>;
  getById: (id: string) => LabelTemplate | undefined;
  clear: () => void;
  applySync: (payload: SyncPayload) => void;

  // Initialize with default template if empty
  ensureDefaultTemplate: () => Promise<void>;
}

interface SyncPayload {
  id: number | string;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: unknown | null;
}

export const useLabelTemplateStore = create<LabelTemplateStore>((set, get) => ({
  // State
  templates: [],
  isLoading: false,
  isLoaded: false,
  error: null,
  lastVersion: 0,

  // Actions
  fetchAll: async (force = false) => {
    const state = get();
    if (state.isLoading) return;
    if (state.isLoaded && !force) return;

    set({ isLoading: true, error: null });
    try {
      const rawTemplates = await getApi().listLabelTemplates();
      const templates = rawTemplates.map((t) => mapApiToFrontend(t as unknown as Record<string, unknown>));
      set({ templates, isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch label templates';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] label_template: fetch failed -', errorMsg);
    }
  },

  createTemplate: async (templateData) => {
    set({ isLoading: true, error: null });
    try {
      const createData = toCreatePayload(templateData);
      const rawCreated = await getApi().createLabelTemplate(createData);
      const created = mapApiToFrontend(rawCreated as unknown as Record<string, unknown>);
      set((state) => ({
        templates: [...state.templates, created],
        isLoading: false,
      }));
      return created;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to create label template';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] label_template: create failed -', errorMsg);
      throw e;
    }
  },

  updateTemplate: async (id, templateData) => {
    set({ isLoading: true, error: null });
    try {
      const updateData = toUpdatePayload(templateData);
      const rawUpdated = await getApi().updateLabelTemplate(id, updateData);
      const updated = mapApiToFrontend(rawUpdated as unknown as Record<string, unknown>);
      set((state) => ({
        templates: state.templates.map((t) => (t.id === id || t.id === updated.id ? updated : t)),
        isLoading: false,
      }));
      return updated;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to update label template';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] label_template: update failed -', errorMsg);
      throw e;
    }
  },

  deleteTemplate: async (id) => {
    set({ isLoading: true, error: null });
    try {
      await getApi().deleteLabelTemplate(id);
      set((state) => ({
        templates: state.templates.filter((t) => t.id !== id),
        isLoading: false,
      }));
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to delete label template';
      set({ error: errorMsg, isLoading: false });
      console.error('[Store] label_template: delete failed -', errorMsg);
      throw e;
    }
  },

  duplicateTemplate: async (template) => {
    const duplicateData: Partial<LabelTemplate> = {
      ...template,
      name: `${template.name} (Copy)`,
      is_default: false,
    };
    return get().createTemplate(duplicateData);
  },

  getById: (id) => get().templates.find((t) => t.id === id),

  clear: () => set({ templates: [], isLoaded: false, error: null, lastVersion: 0 }),

  applySync: (payload: SyncPayload) => {
    const state = get();
    if (!state.isLoaded) return;

    const { id, version, action, data } = payload;

    if (state.lastVersion > 0 && version <= state.lastVersion) return;

    if (state.lastVersion > 0 && version > state.lastVersion + 1) {
      if (!state.isLoading) get().fetchAll(true);
      return;
    }

    const strId = String(id);
    const actualId = strId.includes(':') ? strId.split(':')[1] : strId;

    switch (action) {
      case 'created':
        if (data) {
          const template = mapApiToFrontend(data as Record<string, unknown>);
          const exists = state.templates.some((t) => t.id === actualId || t.id === template.id);
          if (exists) {
            set((s) => ({
              templates: s.templates.map((t) => (t.id === actualId || t.id === template.id ? template : t)),
              lastVersion: version,
            }));
          } else {
            set((s) => ({
              templates: [...s.templates, template],
              lastVersion: version,
            }));
          }
        }
        break;
      case 'updated':
        if (data) {
          const template = mapApiToFrontend(data as Record<string, unknown>);
          set((s) => ({
            templates: s.templates.map((t) => (t.id === actualId || t.id === template.id ? template : t)),
            lastVersion: version,
          }));
        }
        break;
      case 'deleted':
        set((s) => ({
          templates: s.templates.filter((t) => t.id !== actualId),
          lastVersion: version,
        }));
        break;
    }
  },

  ensureDefaultTemplate: async () => {
    await get().fetchAll();
    const { templates } = get();

    if (templates.length === 0) {
      const defaultData: Partial<LabelTemplate> = {
        ...DEFAULT_LABEL_TEMPLATES[0],
        is_default: false,
      };
      await get().createTemplate(defaultData);
    }
  },
}));

// Convenience hooks
export const useLabelTemplates = () => useLabelTemplateStore((state) => state.templates);
export const useLabelTemplatesLoading = () => useLabelTemplateStore((state) => state.isLoading);
export const useLabelTemplateById = (id: string) =>
  useLabelTemplateStore((state) => state.templates.find((t) => t.id === id));

// Action hooks
export const useLabelTemplateActions = () => ({
  fetch: useLabelTemplateStore.getState().fetchAll,
  create: useLabelTemplateStore.getState().createTemplate,
  update: useLabelTemplateStore.getState().updateTemplate,
  delete: useLabelTemplateStore.getState().deleteTemplate,
  duplicate: useLabelTemplateStore.getState().duplicateTemplate,
  ensureDefault: useLabelTemplateStore.getState().ensureDefaultTemplate,
});
