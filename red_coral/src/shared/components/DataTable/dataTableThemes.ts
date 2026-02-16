export interface DataTableTheme {
  headerBg: string;
  headerBorder: string;
  headerText: string;
  rowHover: string;
  rowSelected: string;
  checkboxActive: string;
  checkboxHover: string;
  paginationActive: string;
  selectBtn: string;
  selectModeHeader: string;
  selectModeText: string;
}

export type ThemeColor = 'blue' | 'orange' | 'purple' | 'teal' | 'indigo';

export const THEMES: Record<ThemeColor, DataTableTheme> = {
  blue: {
    headerBg: 'bg-blue-50',
    headerBorder: 'border-blue-100',
    headerText: 'text-blue-700',
    rowHover: 'hover:bg-blue-50/50',
    rowSelected: 'bg-blue-50/70',
    checkboxActive: 'bg-blue-600 border-blue-600',
    checkboxHover: 'hover:border-blue-500',
    paginationActive: 'bg-blue-600 text-white',
    selectBtn: 'text-blue-600 bg-blue-50 hover:bg-blue-100 border-blue-100',
    selectModeHeader: 'bg-blue-50 border-blue-100',
    selectModeText: 'text-blue-700',
  },
  orange: {
    headerBg: 'bg-orange-50',
    headerBorder: 'border-orange-100',
    headerText: 'text-orange-700',
    rowHover: 'hover:bg-orange-50/50',
    rowSelected: 'bg-orange-50/70',
    checkboxActive: 'bg-orange-600 border-orange-600',
    checkboxHover: 'hover:border-orange-500',
    paginationActive: 'bg-orange-600 text-white',
    selectBtn: 'text-orange-600 bg-orange-50 hover:bg-orange-100 border-orange-100',
    selectModeHeader: 'bg-orange-50 border-orange-100',
    selectModeText: 'text-orange-700',
  },
  purple: {
    headerBg: 'bg-purple-50',
    headerBorder: 'border-purple-100',
    headerText: 'text-purple-700',
    rowHover: 'hover:bg-purple-50/50',
    rowSelected: 'bg-purple-50/70',
    checkboxActive: 'bg-purple-600 border-purple-600',
    checkboxHover: 'hover:border-purple-500',
    paginationActive: 'bg-purple-600 text-white',
    selectBtn: 'text-purple-600 bg-purple-50 hover:bg-purple-100 border-purple-100',
    selectModeHeader: 'bg-purple-50 border-purple-100',
    selectModeText: 'text-purple-700',
  },
  teal: {
    headerBg: 'bg-teal-50',
    headerBorder: 'border-teal-100',
    headerText: 'text-teal-700',
    rowHover: 'hover:bg-teal-50/50',
    rowSelected: 'bg-teal-50/70',
    checkboxActive: 'bg-teal-600 border-teal-600',
    checkboxHover: 'hover:border-teal-500',
    paginationActive: 'bg-teal-600 text-white',
    selectBtn: 'text-teal-600 bg-teal-50 hover:bg-teal-100 border-teal-100',
    selectModeHeader: 'bg-teal-50 border-teal-100',
    selectModeText: 'text-teal-700',
  },
  indigo: {
    headerBg: 'bg-indigo-50',
    headerBorder: 'border-indigo-100',
    headerText: 'text-indigo-700',
    rowHover: 'hover:bg-indigo-50/50',
    rowSelected: 'bg-indigo-50/70',
    checkboxActive: 'bg-indigo-600 border-indigo-600',
    checkboxHover: 'hover:border-indigo-500',
    paginationActive: 'bg-indigo-600 text-white',
    selectBtn: 'text-indigo-600 bg-indigo-50 hover:bg-indigo-100 border-indigo-100',
    selectModeHeader: 'bg-indigo-50 border-indigo-100',
    selectModeText: 'text-indigo-700',
  },
};
