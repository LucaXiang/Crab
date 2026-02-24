import { request } from './client';
import type {
  Employee, EmployeeCreate, EmployeeUpdate,
  Zone, ZoneCreate, ZoneUpdate,
  DiningTable, DiningTableCreate, DiningTableUpdate,
  StoreOpResult,
} from '@/core/types/store';

const storePath = (storeId: number, resource: string) =>
  `/api/tenant/stores/${storeId}/${resource}`;

// ── Employees ──
export const listEmployees = (token: string, storeId: number) =>
  request<Employee[]>('GET', storePath(storeId, 'employees'), undefined, token);

export const createEmployee = (token: string, storeId: number, data: EmployeeCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'employees'), data, token);

export const updateEmployee = (token: string, storeId: number, id: number, data: EmployeeUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'employees')}/${id}`, data, token);

export const deleteEmployee = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'employees')}/${id}`, undefined, token);

// ── Zones ──
export const listZones = (token: string, storeId: number) =>
  request<Zone[]>('GET', storePath(storeId, 'zones'), undefined, token);

export const createZone = (token: string, storeId: number, data: ZoneCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'zones'), data, token);

export const updateZone = (token: string, storeId: number, id: number, data: ZoneUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'zones')}/${id}`, data, token);

export const deleteZone = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'zones')}/${id}`, undefined, token);

// ── Dining Tables ──
export const listTables = (token: string, storeId: number) =>
  request<DiningTable[]>('GET', storePath(storeId, 'tables'), undefined, token);

export const createTable = (token: string, storeId: number, data: DiningTableCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'tables'), data, token);

export const updateTable = (token: string, storeId: number, id: number, data: DiningTableUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'tables')}/${id}`, data, token);

export const deleteTable = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'tables')}/${id}`, undefined, token);
