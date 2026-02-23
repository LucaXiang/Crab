import React from 'react';
import { AlertTriangle } from 'lucide-react';

interface DeleteConfirmationProps {
  name?: string;
  entity?: string;
  t: (key: string) => string;
}

export const DeleteConfirmation: React.FC<DeleteConfirmationProps> = ({ name, entity, t }) => {
  const getDescription = () => {
    switch (entity) {
      case 'TABLE':
        return t('settings.table.confirm.delete');
      case 'ZONE':
        return t('settings.table.zone.confirm.delete');
      case 'PRODUCT':
        return t('settings.product.confirm.delete');
      case 'CATEGORY':
        return t('settings.category.confirm.delete');
      default:
        return t('settings.table.confirm.delete');
    }
  };

  return (
    <div className="flex flex-col items-center text-center py-4">
      <div className="w-14 h-14 bg-red-100 rounded-full flex items-center justify-center mb-4">
        <AlertTriangle size={28} className="text-red-600" />
      </div>
      <h3 className="text-lg font-semibold text-gray-900 mb-2">
        {t('common.dialog.confirm_delete')}
      </h3>
      <p className="text-sm text-gray-500 max-w-sm">
        {getDescription()}
      </p>
      {name && (
        <div className="mt-4 px-4 py-2 bg-gray-100 rounded-lg">
          <span className="text-sm font-medium text-gray-700">{name}</span>
        </div>
      )}
    </div>
  );
};
