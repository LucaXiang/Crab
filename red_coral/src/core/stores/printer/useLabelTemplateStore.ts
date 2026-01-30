/**
 * Label Template Store - Fetches and manages label templates from server API
 *
 * Replaces localStorage-based storage with server-side persistence
 */

import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { LabelTemplate, LabelField } from '@/core/domain/types/print';
import type { LabelTemplateCreate, LabelTemplateUpdate } from '@/core/domain/types/api';
import { DEFAULT_LABEL_TEMPLATES } from '@/core/domain/types/print';

// Lazy-load API client to avoid initialization issues
const getApi = () => createTauriClient();

// Map API snake_case to frontend camelCase
function mapApiToFrontend(apiTemplate: Record<string, unknown>): LabelTemplate {
  return {
    id: (apiTemplate.id as string)?.split(':')[1] || apiTemplate.id as string, // Extract ID from "label_template:xxx"
    name: apiTemplate.name as string,
    description: apiTemplate.description as string | undefined,
    width: apiTemplate.width as number,
    height: apiTemplate.height as number,
    padding: apiTemplate.padding as number || 2,
    fields: (apiTemplate.fields as LabelField[]) || [],
    isDefault: apiTemplate.is_default as boolean || false,
    isActive: apiTemplate.is_active as boolean ?? true,
    createdAt: apiTemplate.created_at as number || Date.now(),
    updatedAt: apiTemplate.updated_at as number || Date.now(),
    widthMm: apiTemplate.width as number,
    heightMm: apiTemplate.height as number,
    paddingMmX: apiTemplate.padding_mm_x as number,
    paddingMmY: apiTemplate.padding_mm_y as number,
    renderDpi: apiTemplate.render_dpi as number,
    testData: apiTemplate.test_data as string,
  };
}

// Map frontend camelCase to API snake_case for create
function mapFrontendToApiCreate(template: Partial<LabelTemplate>): LabelTemplateCreate {
  return {
    name: template.name || '',
    description: template.description,
    width: template.widthMm || template.width || 40,
    height: template.heightMm || template.height || 30,
    fields: template.fields || [],
    is_default: template.isDefault || false,
    is_active: template.isActive ?? true,
    padding_mm_x: template.paddingMmX,
    padding_mm_y: template.paddingMmY,
    render_dpi: template.renderDpi,
    test_data: template.testData,
  };
}

// Map frontend camelCase to API snake_case for update
function mapFrontendToApiUpdate(template: Partial<LabelTemplate>): LabelTemplateUpdate {
  const update: LabelTemplateUpdate = {};
  if (template.name !== undefined) update.name = template.name;
  if (template.description !== undefined) update.description = template.description;
  if (template.width !== undefined) update.width = template.width;
  if (template.height !== undefined) update.height = template.height;
  if (template.fields !== undefined) update.fields = template.fields;
  if (template.isDefault !== undefined) update.is_default = template.isDefault;
  if (template.isActive !== undefined) update.is_active = template.isActive;
  if (template.widthMm !== undefined) update.width = template.widthMm;
  if (template.heightMm !== undefined) update.height = template.heightMm;
  if (template.paddingMmX !== undefined) update.padding_mm_x = template.paddingMmX;
  if (template.paddingMmY !== undefined) update.padding_mm_y = template.paddingMmY;
  if (template.renderDpi !== undefined) update.render_dpi = template.renderDpi;
  if (template.testData !== undefined) update.test_data = template.testData;
  return update;
}

interface SyncPayload {
  id: string;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: unknown | null;
}

interface LabelTemplateStore {
  // State
  templates: LabelTemplate[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // Actions
  fetchAll: (force?: boolean) => Promise<void>;
  createTemplate: (template: Partial<LabelTemplate>) => Promise<LabelTemplate>;
  updateTemplate: (id: string, template: Partial<LabelTemplate>) => Promise<LabelTemplate>;
  deleteTemplate: (id: string) => Promise<void>;
  duplicateTemplate: (template: LabelTemplate) => Promise<LabelTemplate>;
  getById: (id: string) => LabelTemplate | undefined;
  clear: () => void;
  applySync: (payload?: SyncPayload) => void;

  // Initialize with default template if empty
  ensureDefaultTemplate: () => Promise<void>;
}

export const useLabelTemplateStore = create<LabelTemplateStore>((set, get) => ({
  // State
  templates: [],
  isLoading: false,
  isLoaded: false,
  error: null,

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
      const createData = mapFrontendToApiCreate(templateData);
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
      const updateData = mapFrontendToApiUpdate(templateData);
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
      isDefault: false,
    };
    return get().createTemplate(duplicateData);
  },

  getById: (id) => get().templates.find((t) => t.id === id),

  clear: () => set({ templates: [], isLoaded: false, error: null }),

  /**
   * Apply sync from server broadcast
   */
  applySync: (payload?: SyncPayload) => {
    console.log('[Store] label_template: received sync signal', payload?.action, payload?.id);
    const state = get();
    if (!state.isLoaded) return;

    if (!payload) {
      // No payload, refetch all
      state.fetchAll(true);
      return;
    }

    const { id, action, data } = payload;
    // Extract actual ID from "label_template:xxx" format
    const actualId = id.includes(':') ? id.split(':')[1] : id;

    switch (action) {
      case 'created':
      case 'updated':
        if (data) {
          const template = mapApiToFrontend(data as Record<string, unknown>);
          set((s) => {
            const exists = s.templates.some((t) => t.id === actualId || t.id === template.id);
            if (exists) {
              return { templates: s.templates.map((t) => (t.id === actualId || t.id === template.id ? template : t)) };
            } else {
              return { templates: [...s.templates, template] };
            }
          });
        } else {
          // No data provided, refetch
          state.fetchAll(true);
        }
        break;
      case 'deleted':
        set((s) => ({ templates: s.templates.filter((t) => t.id !== actualId) }));
        break;
    }
  },

  ensureDefaultTemplate: async () => {
    await get().fetchAll();
    const { templates } = get();

    if (templates.length === 0) {
      // Create a default template from predefined templates
      const defaultData: Partial<LabelTemplate> = {
        ...DEFAULT_LABEL_TEMPLATES[0],
        isDefault: false,
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
