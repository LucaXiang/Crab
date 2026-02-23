import { request } from './client';
import type {
  Employee, EmployeeCreate, EmployeeUpdate,
  Zone, ZoneCreate, ZoneUpdate,
  DiningTable, DiningTableCreate, DiningTableUpdate,
  CatalogOpResult,
} from '@/core/types/catalog';

const catalogPath = (storeId: number, resource: string) =>
  `/api/tenant/stores/${storeId}/catalog/${resource}`;

// ── Employees ──
export const listEmployees = (token: string, storeId: number) =>
  request<Employee[]>('GET', catalogPath(storeId, 'employees'), undefined, token);

export const createEmployee = (token: string, storeId: number, data: EmployeeCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'employees'), data, token);

export const updateEmployee = (token: string, storeId: number, id: number, data: EmployeeUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'employees')}/${id}`, data, token);

export const deleteEmployee = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'employees')}/${id}`, undefined, token);

// ── Zones ──
export const listZones = (token: string, storeId: number) =>
  request<Zone[]>('GET', catalogPath(storeId, 'zones'), undefined, token);

export const createZone = (token: string, storeId: number, data: ZoneCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'zones'), data, token);

export const updateZone = (token: string, storeId: number, id: number, data: ZoneUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'zones')}/${id}`, data, token);

export const deleteZone = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'zones')}/${id}`, undefined, token);

// ── Dining Tables ──
export const listTables = (token: string, storeId: number) =>
  request<DiningTable[]>('GET', catalogPath(storeId, 'tables'), undefined, token);

export const createTable = (token: string, storeId: number, data: DiningTableCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'tables'), data, token);

export const updateTable = (token: string, storeId: number, id: number, data: DiningTableUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'tables')}/${id}`, data, token);

export const deleteTable = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'tables')}/${id}`, undefined, token);
