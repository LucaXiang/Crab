import { create } from 'zustand';
import { Permission, User } from '@/core/domain/types';

interface EscalationRequest {
  requiredPermission: Permission;
  onSuccess: (user: User) => void;
  title?: string;
  description?: string;
}

interface PermissionEscalationStore {
  isOpen: boolean;
  request: EscalationRequest | null;
  
  openEscalation: (req: EscalationRequest) => void;
  closeEscalation: () => void;
}

export const usePermissionEscalationStore = create<PermissionEscalationStore>((set) => ({
  isOpen: false,
  request: null,
  
  openEscalation: (req) => set({ isOpen: true, request: req }),
  closeEscalation: () => set({ isOpen: false, request: null }),
}));
