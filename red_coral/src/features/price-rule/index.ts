/**
 * Price Rule Feature Module
 *
 * 价格规则管理功能模块，包含：
 * - PriceRuleManagement: 价格规则管理页面
 * - PriceRuleWizard: 价格规则创建/编辑向导
 * - store: Zustand store
 */

// Store
export {
  usePriceRuleStore,
  usePriceRules,
  usePriceRulesLoading,
  usePriceRuleById,
  useActivePriceRules,
} from './store';

// Components
export { PriceRuleManagement } from './PriceRuleManagement';
export { PriceRuleWizard } from './PriceRuleWizard';
