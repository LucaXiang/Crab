import React from 'react';
import type { LucideIcon } from 'lucide-react';

interface FormFieldProps {
  label: string;
  children: React.ReactNode;
  required?: boolean;
}

export const FormField: React.FC<FormFieldProps> = ({ label, children, required }) => (
  <div className="space-y-1.5">
    <label className="block text-sm font-medium text-gray-700">
      {label}
      {required && <span className="text-red-500 ml-0.5">*</span>}
    </label>
    {children}
  </div>
);

/**
 * FormSection - 表单分组容器
 * 统一的 section 样式，带图标标题
 */
interface FormSectionProps {
  title: string;
  icon: LucideIcon;
  children: React.ReactNode;
  /** 是否默认折叠 */
  defaultCollapsed?: boolean;
}

export const FormSection: React.FC<FormSectionProps> = ({
  title,
  icon: Icon,
  children,
  defaultCollapsed = false,
}) => {
  const [collapsed, setCollapsed] = React.useState(defaultCollapsed);

  return (
    <section className="bg-white rounded-xl border border-gray-100 p-4 shadow-sm">
      <button
        type="button"
        onClick={() => setCollapsed(!collapsed)}
        className="w-full flex items-center justify-between pb-2 border-b border-gray-100 mb-4 cursor-pointer hover:bg-gray-50 -mx-4 px-4 rounded-t-xl transition-colors"
      >
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900">
          <Icon size={16} className="text-teal-500" />
          {title}
        </h3>
        <svg
          className={`w-4 h-4 text-gray-400 transition-transform ${collapsed ? '' : 'rotate-180'}`}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
        </svg>
      </button>
      {!collapsed && <div className="space-y-4">{children}</div>}
    </section>
  );
};

/**
 * CheckboxField - 带描述的复选框
 */
interface CheckboxFieldProps {
  id: string;
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export const CheckboxField: React.FC<CheckboxFieldProps> = ({
  id,
  label,
  description,
  checked,
  onChange,
  disabled = false,
}) => (
  <div className={`flex items-start space-x-3 py-1 ${disabled ? 'opacity-60' : ''}`}>
    <div className="flex items-center h-5 mt-0.5">
      <input
        type="checkbox"
        id={id}
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        className="w-4 h-4 text-teal-600 rounded border-gray-300 focus:ring-teal-500 disabled:cursor-not-allowed"
      />
    </div>
    <label htmlFor={id} className={`text-gray-700 select-none ${disabled ? 'cursor-not-allowed' : 'cursor-pointer'}`}>
      <span className="font-medium block text-sm">{label}</span>
      {description && (
        <span className="text-xs text-gray-500 block">{description}</span>
      )}
    </label>
  </div>
);

/**
 * SubField - 子选项容器（带左边线缩进）
 */
interface SubFieldProps {
  children: React.ReactNode;
  show?: boolean;
}

export const SubField: React.FC<SubFieldProps> = ({ children, show = true }) => {
  if (!show) return null;
  return (
    <div className="pl-4 border-l-2 border-teal-100 space-y-3 ml-2">
      {children}
    </div>
  );
};

export const inputClass =
  'w-full px-3 py-2.5 border border-gray-200 rounded-xl text-sm bg-white focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-500 transition-colors placeholder:text-gray-400';

export const selectClass =
  'w-full px-3 py-2.5 border border-gray-200 rounded-xl text-sm bg-white focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-500 transition-colors appearance-none cursor-pointer';
