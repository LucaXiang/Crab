import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Global mock: Tauri APIs are unavailable in jsdom test environment.
// Without these mocks, any transitive import chain reaching @tauri-apps/*
// causes the worker to hang and OOM (e.g., PaymentFlow → SelectModePage
// → sendCommand → tauri-client → @tauri-apps/api/core).

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => path),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    setFullscreen: vi.fn(),
    onCloseRequested: vi.fn(() => Promise.resolve(() => {})),
  })),
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
  save: vi.fn(),
  confirm: vi.fn(() => Promise.resolve(false)),
  message: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));
