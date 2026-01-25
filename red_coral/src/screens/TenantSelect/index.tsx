import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Building2, Plus, Trash2, ChevronRight, Power, AlertCircle } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useBridgeStore, type TenantInfo } from '@/core/stores/bridge';

export const TenantSelectScreen: React.FC = () => {
  const navigate = useNavigate();
  const {
    tenants,
    fetchTenants,
    switchTenant,
    removeTenant,
    isLoading,
    error,
  } = useBridgeStore();

  const [selectedTenant, setSelectedTenant] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<string | null>(null);
  const [actionError, setActionError] = useState('');

  useEffect(() => {
    fetchTenants();
  }, [fetchTenants]);

  const handleSelectTenant = async (tenantId: string) => {
    setActionError('');
    try {
      await switchTenant(tenantId);
      navigate('/login', { replace: true });
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : 'Failed to switch tenant');
    }
  };

  const handleDeleteTenant = async (tenantId: string) => {
    setActionError('');
    try {
      await removeTenant(tenantId);
      setShowDeleteConfirm(null);
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : 'Failed to remove tenant');
    }
  };

  const handleAddTenant = () => {
    navigate('/setup', { replace: true });
  };

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      {/* Close Button */}
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title="Close Application"
      >
        <Power size={24} />
      </button>

      <div className="w-full max-w-2xl space-y-8">
        {/* Header */}
        <div className="text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-[#FF5E5E]/10 rounded-2xl mb-4">
            <Building2 className="text-[#FF5E5E]" size={32} />
          </div>
          <h1 className="text-3xl font-bold text-gray-900 mb-2">
            Select Tenant
          </h1>
          <p className="text-gray-500">
            Choose a tenant to continue, or add a new one
          </p>
        </div>

        {/* Error Message */}
        {(actionError || error) && (
          <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
            <AlertCircle size={20} className="shrink-0" />
            <span className="text-sm font-medium">{actionError || error}</span>
          </div>
        )}

        {/* Tenant List */}
        <div className="space-y-3">
          {tenants.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              <Building2 size={48} className="mx-auto mb-4 text-gray-300" />
              <p>No tenants activated yet</p>
            </div>
          ) : (
            tenants.map((tenant) => (
              <div
                key={tenant.tenant_id}
                className={`group relative p-6 rounded-2xl border-2 bg-white transition-all ${
                  selectedTenant === tenant.tenant_id
                    ? 'border-[#FF5E5E] ring-4 ring-[#FF5E5E]/10'
                    : 'border-gray-200 hover:border-gray-300'
                }`}
              >
                <div className="flex items-center gap-4">
                  {/* Tenant Icon */}
                  <div className="w-12 h-12 rounded-xl bg-gray-100 flex items-center justify-center">
                    <Building2 className="text-gray-600" size={24} />
                  </div>

                  {/* Tenant Info */}
                  <div className="flex-1 min-w-0">
                    <h3 className="text-lg font-semibold text-gray-900 truncate">
                      {tenant.tenant_name || tenant.tenant_id}
                    </h3>
                    <div className="flex items-center gap-2 text-sm text-gray-500">
                      <span className="truncate">{tenant.tenant_id}</span>
                      {tenant.has_certificates && (
                        <span className="px-2 py-0.5 bg-green-100 text-green-700 rounded text-xs">
                          Activated
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-2">
                    {/* Delete Button */}
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setShowDeleteConfirm(tenant.tenant_id);
                      }}
                      className="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg opacity-0 group-hover:opacity-100 transition-all"
                      title="Remove Tenant"
                    >
                      <Trash2 size={20} />
                    </button>

                    {/* Select Button */}
                    <button
                      onClick={() => handleSelectTenant(tenant.tenant_id)}
                      disabled={isLoading}
                      className="flex items-center gap-2 px-4 py-2 bg-[#FF5E5E] text-white rounded-lg hover:bg-[#E54545] transition-colors disabled:opacity-50"
                    >
                      <span>Select</span>
                      <ChevronRight size={16} />
                    </button>
                  </div>
                </div>

                {/* Delete Confirmation */}
                {showDeleteConfirm === tenant.tenant_id && (
                  <div className="absolute inset-0 bg-white/95 rounded-2xl flex items-center justify-center gap-4 p-6">
                    <span className="text-gray-600">Remove this tenant?</span>
                    <button
                      onClick={() => setShowDeleteConfirm(null)}
                      className="px-4 py-2 bg-gray-100 text-gray-600 rounded-lg hover:bg-gray-200"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={() => handleDeleteTenant(tenant.tenant_id)}
                      className="px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600"
                    >
                      Remove
                    </button>
                  </div>
                )}
              </div>
            ))
          )}
        </div>

        {/* Add Tenant Button */}
        <button
          onClick={handleAddTenant}
          className="w-full p-6 rounded-2xl border-2 border-dashed border-gray-300 hover:border-[#FF5E5E] hover:bg-red-50 transition-all flex items-center justify-center gap-3 text-gray-500 hover:text-[#FF5E5E]"
        >
          <Plus size={24} />
          <span className="font-medium">Add New Tenant</span>
        </button>
      </div>
    </div>
  );
};

export default TenantSelectScreen;
