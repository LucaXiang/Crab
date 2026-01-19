import React from 'react';
import { Plus, LucideIcon } from 'lucide-react';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';

interface ManagementHeaderProps {
  icon: LucideIcon;
  title: string;
  description: string;
  addButtonText: string;
  onAdd: () => void;
  themeColor?: 'blue' | 'purple' | 'orange' | 'teal' | 'indigo';
  /**
   * Optional permission required to show the add button
   * If not provided, button is always shown
   */
  permission?: Permission;
}

const colorClasses = {
  blue: {
    iconBg: 'bg-blue-100',
    iconText: 'text-blue-600',
    button: 'bg-blue-600 shadow-blue-600/20 hover:bg-blue-700 hover:shadow-blue-600/30'
  },
  purple: {
    iconBg: 'bg-purple-100',
    iconText: 'text-purple-600',
    button: 'bg-purple-600 shadow-purple-600/20 hover:bg-purple-700 hover:shadow-purple-600/30'
  },
  orange: {
    iconBg: 'bg-orange-100',
    iconText: 'text-orange-600',
    button: 'bg-orange-600 shadow-orange-600/20 hover:bg-orange-700 hover:shadow-orange-600/30'
  },
  teal: {
    iconBg: 'bg-teal-100',
    iconText: 'text-teal-600',
    button: 'bg-teal-600 shadow-teal-600/20 hover:bg-teal-700 hover:shadow-teal-600/30'
  },
  indigo: {
    iconBg: 'bg-indigo-100',
    iconText: 'text-indigo-600',
    button: 'bg-indigo-600 shadow-indigo-600/20 hover:bg-indigo-700 hover:shadow-indigo-600/30'
  }
};

export const ManagementHeader: React.FC<ManagementHeaderProps> = ({
  icon: Icon,
  title,
  description,
  addButtonText,
  onAdd,
  themeColor = 'blue',
  permission
}) => {
  const colors = colorClasses[themeColor];

  const addButton = (
    <button
      onClick={onAdd}
      className={`inline-flex items-center gap-2 px-4 py-2.5 ${colors.button} text-white rounded-xl text-sm font-semibold shadow-lg transition-all`}
    >
      <Plus size={16} />
      <span>{addButtonText}</span>
    </button>
  );

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className={`w-10 h-10 ${colors.iconBg} rounded-xl flex items-center justify-center`}>
            <Icon size={20} className={colors.iconText} />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{title}</h2>
            <p className="text-sm text-gray-500">{description}</p>
          </div>
        </div>
        {permission ? (
          <ProtectedGate permission={permission}>
            {addButton}
          </ProtectedGate>
        ) : (
          addButton
        )}
      </div>
    </div>
  );
};
