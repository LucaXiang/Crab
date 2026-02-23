/**
 * Label Template Store - Fetches and manages label templates from server API
 *
 * 前端 LabelField 直接使用 field_id/field_type，与 Rust 后端 serde 完全对齐，无映射层
 */

import { create } from 'zustand';
import { logger } from '@/utils/logger';
import { createTauriClient } from '@/infrastructure/api';
import type { LabelTemplate } from '@/core/domain/types/print';
import type { LabelTemplateCreate, LabelTemplateUpdate } from '@/core/domain/types/api';
import { DEFAULT_LABEL_TEMPLATES } from '@/core/domain/types/print';

// Lazy-load API client to avoid initialization issues
const getApi = () => createTauriClient();

/** 补全 API 响应中可能缺失的默认值 */
function normalizeTemplate(t: LabelTemplate): LabelTemplate {
  return {
    ...t,
    padding: t.padding || 2,
    fields: (t.fields || []).map(f => ({
      ...f,
      // 后端 LabelField 有 id/template_id (DB 行号)，前端不关心，但不需要剥离
      field_type: f.field_type || 'text',
    })),
    is_default: t.is_default || false,
    is_active: t.is_active ?? true,
    created_at: t.created_at || Date.now(),
    updated_at: t.updated_at || Date.now(),
    width_mm: t.width_mm ?? t.width,
    height_mm: t.height_mm ?? t.height,
  };
}

/** 从 LabelField[] 中剥离 _pending_image_path (编辑器临时字段，不入库) */
function stripEditorFields(fields: LabelTemplate['fields']) {
  return fields.map(({ _pending_image_path, ...rest }) => rest);
}

// 构造 Create payload
function toCreatePayload(template: Partial<LabelTemplate>): LabelTemplateCreate {
  return {
    name: template.name || '',
    description: template.description,
    width: template.width_mm || template.width || 40,
    height: template.height_mm || template.height || 30,
    fields: stripEditorFields(template.fields || []),
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
  if (template.fields !== undefined) update.fields = stripEditorFields(template.fields);
  if (template.is_default !== undefined) update.is_default = template.is_default;
  if (template.is_active !== undefined) update.is_active = template.is_active;
  // width_mm 是前端编辑器使用的字段名，对应后端 width
  if (template.width_mm !== undefined) update.width = template.width_mm;
  else if (template.width !== undefined) update.width = template.width;
  if (template.height_mm !== undefined) update.height = template.height_mm;
  else if (template.height !== undefined) update.height = template.height;
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
  updateTemplate: (id: number, template: Partial<LabelTemplate>) => Promise<LabelTemplate>;
  deleteTemplate: (id: number) => Promise<void>;
  duplicateTemplate: (template: LabelTemplate) => Promise<LabelTemplate>;
  getById: (id: number) => LabelTemplate | undefined;
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
      const templates = rawTemplates.map(normalizeTemplate);
      set({ templates, isLoading: false, isLoaded: true });
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to fetch label templates';
      set({ error: errorMsg, isLoading: false });
      logger.error('Label template fetch failed', undefined, { component: 'LabelTemplateStore', detail: errorMsg });
    }
  },

  createTemplate: async (templateData) => {
    set({ isLoading: true, error: null });
    try {
      const createData = toCreatePayload(templateData);
      const rawCreated = await getApi().createLabelTemplate(createData);
      const created = normalizeTemplate(rawCreated);
      // 去重：sync 事件可能先于 API 响应到达，已经添加过
      set((state) => {
        const exists = state.templates.some((t) => t.id === created.id);
        return {
          templates: exists
            ? state.templates.map((t) => (t.id === created.id ? created : t))
            : [...state.templates, created],
          isLoading: false,
        };
      });
      return created;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to create label template';
      set({ error: errorMsg, isLoading: false });
      logger.error('Label template create failed', undefined, { component: 'LabelTemplateStore', detail: errorMsg });
      throw e;
    }
  },

  updateTemplate: async (id, templateData) => {
    set({ isLoading: true, error: null });
    try {
      const updateData = toUpdatePayload(templateData);
      const rawUpdated = await getApi().updateLabelTemplate(id, updateData);
      const updated = normalizeTemplate(rawUpdated);
      set((state) => ({
        templates: state.templates.map((t) => (t.id === id || t.id === updated.id ? updated : t)),
        isLoading: false,
      }));
      return updated;
    } catch (e: unknown) {
      const errorMsg = e instanceof Error ? e.message : 'Failed to update label template';
      set({ error: errorMsg, isLoading: false });
      logger.error('Label template update failed', undefined, { component: 'LabelTemplateStore', detail: errorMsg });
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
      logger.error('Label template delete failed', undefined, { component: 'LabelTemplateStore', detail: errorMsg });
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

    const actualId = Number(id);

    switch (action) {
      case 'created':
        if (data) {
          const template = normalizeTemplate(data as LabelTemplate);
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
          const template = normalizeTemplate(data as LabelTemplate);
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
      for (const preset of DEFAULT_LABEL_TEMPLATES) {
        await get().createTemplate(preset);
      }
    }
  },
}));

// Convenience hooks
export const useLabelTemplates = () => useLabelTemplateStore((state) => state.templates);
export const useLabelTemplatesLoading = () => useLabelTemplateStore((state) => state.isLoading);
export const useLabelTemplateById = (id: number) =>
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
