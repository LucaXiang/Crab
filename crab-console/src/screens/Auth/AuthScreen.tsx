import React, { useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { Spinner } from '@/presentation/components/ui/Spinner';

export const AuthScreen: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const setAuth = useAuthStore(s => s.setAuth);

  useEffect(() => {
    const token = searchParams.get('token');
    const tenantId = searchParams.get('tenant_id');

    if (token && tenantId) {
      setAuth(token, null, tenantId);
      navigate('/', { replace: true });
    } else {
      window.location.href = 'https://redcoral.app/login';
    }
  }, [searchParams, setAuth, navigate]);

  return (
    <div className="flex items-center justify-center min-h-screen">
      <Spinner className="w-8 h-8 text-primary-500" />
    </div>
  );
};
