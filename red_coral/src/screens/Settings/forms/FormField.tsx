import React from 'react';

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

export const inputClass =
  'w-full px-3 py-2.5 border border-gray-200 rounded-xl text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors placeholder:text-gray-400';

export const selectClass =
  'w-full px-3 py-2.5 border border-gray-200 rounded-xl text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors appearance-none cursor-pointer';
