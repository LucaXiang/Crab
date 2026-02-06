/**
 * Z-Index 层级常量
 *
 * 统一管理所有 Modal、Toast、Overlay 的 z-index 值,避免硬编码和层级冲突。
 *
 * 使用规范:
 * - 使用 `z-${Z_INDEX.MODAL_BASE}` 而非硬编码数值
 * - 新增 Modal 时参考此层级系统选择合适的 z-index
 */

export const Z_INDEX = {
  /** 普通 Modal - 信息展示、订单详情 */
  MODAL_BASE: 50,

  /** 业务 Modal - POS 操作、支付流程、快速添加 */
  MODAL_BUSINESS: 60,

  /** 管理 Modal - Settings CRUD (商品、分类、餐桌等) */
  MODAL_MANAGEMENT: 80,

  /** 嵌套确认弹窗 - 未保存提示、加载失败提示 */
  MODAL_NESTED: 90,

  /** 特殊配置器 - 商品属性配置、复杂表单 */
  MODAL_CONFIGURATOR: 100,

  /** 权限升级 - 主管授权 (全局最高优先级) */
  MODAL_AUTH: 9999,

  /** 虚拟键盘 - 在 Toast 之上，确保所有弹窗中都可用 */
  VIRTUAL_KEYBOARD: 10001,

  /** Toast 通知 - 始终位于最上层 */
  TOAST: 10002,
} as const;

/**
 * 类型推导: Z_INDEX 的值类型
 */
export type ZIndexValue = (typeof Z_INDEX)[keyof typeof Z_INDEX];

/**
 * 工具函数: 获取 Tailwind z-index 类名
 *
 * @example
 * getZIndexClass(Z_INDEX.MODAL_BASE) // 'z-50'
 * getZIndexClass(Z_INDEX.MODAL_AUTH) // 'z-[9999]'
 */
export function getZIndexClass(zIndex: ZIndexValue): string {
  // Tailwind 内置 z-index: 0, 10, 20, 30, 40, 50
  // 超出范围使用 z-[value] 语法
  if (zIndex <= 50 && zIndex % 10 === 0) {
    return `z-${zIndex}`;
  }
  return `z-[${zIndex}]`;
}
