import React from 'react';
import { usePermissionEscalationStore } from '@/core/stores/auth/usePermissionEscalationStore';
import { SupervisorAuthModal } from './SupervisorAuthModal';

export const PermissionEscalationProvider: React.FC = () => {
  const { isOpen, request, closeEscalation } = usePermissionEscalationStore();

  if (!isOpen || !request) return null;

  return (
    <SupervisorAuthModal
      isOpen={isOpen}
      onClose={closeEscalation}
      onSuccess={(user) => {
        request.onSuccess(user);
        // SupervisorAuthModal closes itself? No, it calls onClose which calls closeEscalation.
        // Wait, SupervisorAuthModal implementation:
        // onSuccess(supervisor);
        // onClose();
        // So it calls onSuccess, then onClose.
        // If we pass closeEscalation as onClose, it will be called.
      }}
      requiredPermission={request.requiredPermission}
      actionDescription={request.description}
    />
  );
};
