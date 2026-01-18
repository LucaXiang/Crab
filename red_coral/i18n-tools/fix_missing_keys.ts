#!/usr/bin/env -S deno run --allow-read --allow-write

import { parse } from "https://deno.land/std@0.208.0/path/mod.ts";

// 缺失的 keys 和规范化建议
const missingKeys: Record<string, { standardKey: string; translation: string; reason?: string }> = {
  // === common 组 ===
  "common.exported": { standardKey: "common.exported", translation: "已导出", reason: "通用状态" },
  "common.retry": { standardKey: "common.retry", translation: "重试", reason: "通用操作" },
  "common.date": { standardKey: "common.date", translation: "日期", reason: "通用字段" },
  "common.export": { standardKey: "common.export", translation: "导出", reason: "通用操作" },
  "common.noResults": { standardKey: "common.noResults", translation: "无结果", reason: "通用空状态" },
  "common.enabled": { standardKey: "common.enabled", translation: "已启用", reason: "通用状态" },
  "common.invalidForm": { standardKey: "common.invalidForm", translation: "表单无效", reason: "通用验证" },
  "common.saveSuccess": { standardKey: "common.saveSuccess", translation: "保存成功", reason: "通用消息" },
  "common.saveFailed": { standardKey: "common.saveFailed", translation: "保存失败", reason: "通用消息" },
  "common.refresh": { standardKey: "common.refresh", translation: "刷新", reason: "通用操作" },
  "common.print": { standardKey: "common.print", translation: "打印", reason: "通用操作" },
  "common.tapToClose": { standardKey: "common.tapToClose", translation: "点击关闭", reason: "通用提示" },
  "common.unknownItem": { standardKey: "common.unknownItem", translation: "未知项目", reason: "通用状态" },
  "common.processing": { standardKey: "common.processing", translation: "处理中", reason: "通用状态" },
  "common.to": { standardKey: "common.to", translation: "至", reason: "通用分隔符" },
  "common.of": { standardKey: "common.of", translation: "/", reason: "通用分隔符" },
  "common.entries": { standardKey: "common.entries", translation: "条", reason: "通用分页" },
  "common.page": { standardKey: "common.page", translation: "页", reason: "通用分页" },

  // === statistics 组 ===
  "statistics.salesReport": { standardKey: "statistics.report.sales", translation: "销售报告", reason: "报告类型" },
  "statistics.productReport": { standardKey: "statistics.report.product", translation: "商品报告", reason: "报告类型" },
  "statistics.categoryReport": { standardKey: "statistics.report.category", translation: "分类报告", reason: "报告类型" },
  "statistics.detailedReportComingSoon": { standardKey: "statistics.report.detailedComingSoon", translation: "详细报告即将推出", reason: "报告提示" },
  "statistics.revenueTrend": { standardKey: "statistics.chart.revenueTrend", translation: "收入趋势", reason: "图表标题" },
  "statistics.averageOrderValue": { standardKey: "statistics.metric.avgOrderValue", translation: "客单价", reason: "统计指标" },
  "statistics.cashRevenue": { standardKey: "statistics.metric.cashRevenue", translation: "现金收入", reason: "统计指标" },
  "statistics.cardRevenue": { standardKey: "statistics.metric.cardRevenue", translation: "银行卡收入", reason: "统计指标" },
  "statistics.otherRevenue": { standardKey: "statistics.metric.otherRevenue", translation: "其他收入", reason: "统计指标" },
  "statistics.avgGuestSpend": { standardKey: "statistics.metric.avgGuestSpend", translation: "人均消费", reason: "统计指标" },
  "statistics.avgDiningTime": { standardKey: "statistics.metric.avgDiningTime", translation: "平均用餐时间", reason: "统计指标" },
  "statistics.voidedOrders": { standardKey: "statistics.metric.voidedOrders", translation: "作废订单数", reason: "统计指标" },
  "statistics.totalDiscount": { standardKey: "statistics.metric.totalDiscount", translation: "总折扣", reason: "统计指标" },
  "statistics.salesByCategory": { standardKey: "statistics.chart.salesByCategory", translation: "分类销售", reason: "图表标题" },
  "statistics.status.all": { standardKey: "common.status.all", translation: "全部", reason: "通用状态" },
  "statistics.status.COMPLETED": { standardKey: "statistics.status.completed", translation: "已完成", reason: "订单状态" },
  "statistics.status.VOIDED": { standardKey: "statistics.status.voided", translation: "已作废", reason: "订单状态" },
  "statistics.status.MERGED": { standardKey: "statistics.status.merged", translation: "已合并", reason: "订单状态" },

  // === auth 组 ===
  "auth.unauthorized.title": { standardKey: "auth.unauthorized.title", translation: "未授权访问", reason: "页面标题" },
  "auth.unauthorized.message": { standardKey: "auth.unauthorized.message", translation: "您没有权限访问此页面", reason: "提示消息" },
  "auth.unauthorized.contact": { standardKey: "auth.unauthorized.contact", translation: "请联系管理员", reason: "联系信息" },
  "auth.unauthorized.hint": { standardKey: "auth.unauthorized.hint", translation: "当前用户无权执行此操作", reason: "提示信息" },
  "auth.currentUser": { standardKey: "auth.currentUser", translation: "当前用户", reason: "用户信息" },
  "auth.login.title": { standardKey: "auth.login.title", translation: "登录", reason: "页面标题" },
  "auth.login.subtitle": { standardKey: "auth.login.subtitle", translation: "欢迎使用 RedCoral POS", reason: "副标题" },
  "auth.login.subtitleDesc": { standardKey: "auth.login.subtitleDesc", translation: "请输入您的凭据以继续", reason: "描述" },
  "auth.login.enterDetails": { standardKey: "auth.login.enterDetails", translation: "请输入登录信息", reason: "提示" },
  "auth.login.username": { standardKey: "auth.login.username", translation: "用户名", reason: "表单字段" },
  "auth.login.usernamePlaceholder": { standardKey: "auth.login.usernamePlaceholder", translation: "请输入用户名", reason: "占位符" },
  "auth.login.password": { standardKey: "auth.login.password", translation: "密码", reason: "表单字段" },
  "auth.login.passwordPlaceholder": { standardKey: "auth.login.passwordPlaceholder", translation: "请输入密码", reason: "占位符" },
  "auth.login.submit": { standardKey: "auth.login.submit", translation: "登录", reason: "按钮" },
  "auth.login.error.emptyFields": { standardKey: "auth.login.error.emptyFields", translation: "请填写所有字段", reason: "错误消息" },
  "auth.login.error.invalidCredentials": { standardKey: "auth.login.error.invalidCredentials", translation: "用户名或密码错误", reason: "错误消息" },
  "auth.login.feature.multiZone": { standardKey: "auth.login.feature.multiZone", translation: "多区域管理", reason: "功能特性" },
  "auth.login.feature.multiZoneDesc": { standardKey: "auth.login.feature.multiZoneDesc", translation: "支持多个营业区域", reason: "功能描述" },
  "auth.login.feature.fastCheckout": { standardKey: "auth.login.feature.fastCheckout", translation: "快速结账", reason: "功能特性" },
  "auth.login.feature.fastCheckoutDesc": { standardKey: "auth.login.feature.fastCheckoutDesc", translation: "高效收银体验", reason: "功能描述" },
  "auth.supervisorApproval": { standardKey: "auth.supervisorApproval", translation: "需要主管审批", reason: "权限提示" },

  // === checkout 组 ===
  "checkout.voidReason.runaway": { standardKey: "checkout.voidReason.runaway", translation: "顾客逃单", reason: "作废原因" },
  "checkout.voidReason.dineAndDash": { standardKey: "checkout.voidReason.dineAndDash", translation: "吃完就跑", reason: "作废原因" },
  "checkout.voidReason.systemTest": { standardKey: "checkout.voidReason.systemTest", translation: "系统测试", reason: "作废原因" },
  "checkout.voidReason.ownerTreat": { standardKey: "checkout.voidReason.ownerTreat", translation: "老板请客", reason: "作废原因" },
  "checkout.items.unpaid": { standardKey: "checkout.items.unpaid", translation: "未付项目", reason: "订单状态" },
  "checkout.timeline.title": { standardKey: "checkout.timeline.title", translation: "操作记录", reason: "区域标题" },

  // === timeline 组 ===
  "timeline.event.itemRestored": { standardKey: "timeline.event.itemRestored", translation: "恢复项目", reason: "事件类型" },
  "timeline.event.paymentCancelled": { standardKey: "timeline.event.paymentCancelled", translation: "取消支付", reason: "事件类型" },
  "timeline.event.orderRestored": { standardKey: "timeline.event.orderRestored", translation: "恢复订单", reason: "事件类型" },
  "timeline.surchargeExempt": { standardKey: "timeline.surchargeExempt", translation: "免除附加费", reason: "事件描述" },
  "timeline.orderMerged": { standardKey: "timeline.event.orderMerged", translation: "合并订单", reason: "事件类型" },
  "timeline.orderMoved": { standardKey: "timeline.event.orderMoved", translation: "转移订单", reason: "事件类型" },
  "timeline.orderMovedOut": { standardKey: "timeline.event.orderMovedOut", translation: "转出订单", reason: "事件类型" },
  "timeline.orderMergedOut": { standardKey: "timeline.event.orderMergedOut", translation: "转出合并", reason: "事件类型" },
  "timeline.prePaymentPrinted": { standardKey: "timeline.event.prePaymentPrinted", translation: "预付小票", reason: "事件类型" },
  "timeline.prePaymentNote": { standardKey: "timeline.event.prePaymentNote", translation: "预付备注", reason: "事件类型" },

  // === table 组 ===
  "table.ghostOrders": { standardKey: "table.ghostOrders", translation: "幽灵订单", reason: "订单状态" },
  "table.addItems": { standardKey: "table.addItems", translation: "加单", reason: "操作" },
  "table.openTable": { standardKey: "table.openTable", translation: "开台", reason: "操作" },
  "table.confirmAdd": { standardKey: "table.confirmAdd", translation: "确认加单", reason: "按钮" },
  "table.confirmOpen": { standardKey: "table.confirmOpen", translation: "确认开台", reason: "按钮" },
  "table.prePayment": { standardKey: "table.filter.prePayment", translation: "预付", reason: "筛选条件" },

  // === history.info 组 ===
  "history.info.orderId": { standardKey: "history.info.orderId", translation: "订单号", reason: "信息字段" },
  "history.info.status": { standardKey: "history.info.status", translation: "状态", reason: "信息字段" },

  // === settings 组 ===
  "settings.form.basicInfo": { standardKey: "settings.form.basicInfo", translation: "基本信息", reason: "表单标题" },
  "settings.form.extendedInfo": { standardKey: "settings.form.extendedInfo", translation: "扩展信息", reason: "表单标题" },
  "settings.allAttributesSelected": { standardKey: "settings.allAttributesSelected", translation: "已选择所有属性", reason: "提示信息" },
  "settings.zone.form.name": { standardKey: "settings.zone.form.name", translation: "区域名称", reason: "表单字段" },
  "settings.zone.unit": { standardKey: "settings.zone.unit", translation: "个区域", reason: "单位" },
  "settings.zone.noData": { standardKey: "settings.zone.noData", translation: "暂无区域", reason: "空状态" },
  "settings.zone.action.add": { standardKey: "settings.zone.action.add", translation: "添加区域", reason: "操作" },
  "settings.zone.action.edit": { standardKey: "settings.zone.action.edit", translation: "编辑区域", reason: "操作" },
  "settings.zone.action.delete": { standardKey: "settings.zone.action.delete", translation: "删除区域", reason: "操作" },
  "settings.zone.action.deleted": { standardKey: "settings.zone.action.deleted", translation: "区域已删除", reason: "消息" },
  "settings.zone.deleteBlocked": { standardKey: "settings.zone.deleteBlocked", translation: "无法删除：区域包含桌台", reason: "消息" },
  "settings.zone.deleteFailed": { standardKey: "settings.zone.deleteFailed", translation: "删除区域失败", reason: "消息" },
  "settings.zoneCreated": { standardKey: "settings.zone.message.created", translation: "区域创建成功", reason: "消息" },
  "settings.zoneUpdated": { standardKey: "settings.zone.message.updated", translation: "区域更新成功", reason: "消息" },
  "settings.tableCreated": { standardKey: "settings.table.message.created", translation: "桌台创建成功", reason: "消息" },
  "settings.tableUpdated": { standardKey: "settings.table.message.updated", translation: "桌台更新成功", reason: "消息" },
  "settings.productCreated": { standardKey: "settings.product.message.created", translation: "商品创建成功", reason: "消息" },
  "settings.productUpdated": { standardKey: "settings.product.message.updated", translation: "商品更新成功", reason: "消息" },
  "settings.categoryExists": { standardKey: "settings.category.message.exists", translation: "分类已存在", reason: "消息" },
  "settings.category.deleteBlocked": { standardKey: "settings.category.deleteBlocked", translation: "无法删除：分类包含商品", reason: "消息" },
  "settings.category.deleteFailed": { standardKey: "settings.category.deleteFailed", translation: "删除分类失败", reason: "消息" },
  "settings.externalIdRequired": { standardKey: "settings.externalIdRequired", translation: "请输入编号", reason: "验证" },
  "settings.externalIdExists": { standardKey: "settings.externalIdExists", translation: "编号已存在", reason: "验证" },
  "settings.unsavedConfirm": { standardKey: "settings.unsavedConfirm", translation: "有未保存的更改", reason: "确认" },
  "settings.unsavedConfirmHint": { standardKey: "settings.unsavedConfirmHint", translation: "确定要离开吗？", reason: "确认" },
  "settings.table.zone.confirm.delete": { standardKey: "settings.table.zone.confirm.delete", translation: "确定删除？", reason: "确认" },
  "settings.table.zone.form.name": { standardKey: "settings.table.zone.form.name", translation: "区域名称", reason: "表单" },
  "settings.table.zone.form.namePlaceholder": { standardKey: "settings.table.zone.form.namePlaceholder", translation: "请输入区域名称", reason: "占位符" },
  "settings.table.zone.form.surchargeType": { standardKey: "settings.table.zone.form.surchargeType", translation: "附加费类型", reason: "表单" },
  "settings.table.zone.form.fixedPerItem": { standardKey: "settings.table.zone.form.surchargeFixed", translation: "每项固定", reason: "选项" },
  "settings.table.zone.form.percentage": { standardKey: "settings.table.zone.form.surchargePercentage", translation: "百分比", reason: "选项" },
  "settings.table.zone.form.surchargeAmount": { standardKey: "settings.table.zone.form.surchargeAmount", translation: "附加费金额", reason: "表单" },

  // === settings.attribute 组 ===
  "settings.attribute.option.noData": { standardKey: "settings.attribute.option.noData", translation: "暂无选项", reason: "空状态" },

  // === settings.printer 组 ===
  "settings.printer.kitchenPrinting": { standardKey: "settings.printer.kitchenPrinting", translation: "厨房打印", reason: "功能" },
  "settings.printer.routingSystem.levelProduct": { standardKey: "settings.printer.routingSystem.levelProduct", translation: "商品优先级", reason: "路由" },
  "settings.printer.routingSystem.levelCategory": { standardKey: "settings.printer.routingSystem.levelCategory", translation: "分类优先级", reason: "路由" },
  "settings.printer.routingSystem.levelGlobal": { standardKey: "settings.printer.routingSystem.levelGlobal", translation: "全局优先级", reason: "路由" },
  "settings.kitchenPrinter": { standardKey: "settings.kitchenPrinter", translation: "厨房打印机", reason: "设备" },
  "common.addStation": { standardKey: "settings.printer.addStation", translation: "添加工位", reason: "操作" },

  // === settings.user 组 ===
  "settings.passwordRequired": { standardKey: "settings.user.form.passwordRequired", translation: "请输入密码", reason: "验证" },
  "settings.passwordTooShort": { standardKey: "settings.user.form.passwordTooShort", translation: "密码至少6位", reason: "验证" },
  "settings.passwordMismatch": { standardKey: "settings.user.form.passwordMismatch", translation: "两次输入不一致", reason: "验证" },
  "settings.resetPasswordSuccess": { standardKey: "settings.user.message.resetPasswordSuccess", translation: "密码重置成功", reason: "消息" },
  "settings.resetPasswordFailed": { standardKey: "settings.user.message.resetPasswordFailed", translation: "密码重置失败", reason: "消息" },
  "settings.resetPassword": { standardKey: "settings.resetPassword", translation: "重置密码", reason: "操作" },
  "settings.newPassword": { standardKey: "settings.newPassword", translation: "新密码", reason: "表单" },
  "settings.enterNewPassword": { standardKey: "settings.enterNewPassword", translation: "输入新密码", reason: "提示" },
  "settings.passwordMinLength": { standardKey: "settings.passwordMinLength", translation: "最小6位", reason: "提示" },
  "settings.confirmPassword": { standardKey: "settings.confirmPassword", translation: "确认密码", reason: "表单" },
  "settings.confirmPasswordPlaceholder": { standardKey: "settings.confirmPasswordPlaceholder", translation: "请再次输入密码", reason: "占位符" },
  "settings.resetPasswordWarning": { standardKey: "settings.resetPasswordWarning", translation: "密码将立即生效", reason: "提示" },
  "settings.user.form.usernameRequired": { standardKey: "settings.user.form.usernameRequired", translation: "请输入用户名", reason: "验证" },
  "settings.user.form.passwordRequired": { standardKey: "settings.user.form.passwordRequired", translation: "请输入密码", reason: "验证" },
  "settings.user.form.displayNameRequired": { standardKey: "settings.user.form.displayNameRequired", translation: "请输入显示名称", reason: "验证" },
  "settings.user.action.updateSuccess": { standardKey: "settings.user.message.updateSuccess", translation: "用户更新成功", reason: "消息" },
  "settings.user.action.createSuccess": { standardKey: "settings.user.message.createSuccess", translation: "用户创建成功", reason: "消息" },

  // === settings.reorderFailed ===
  "settings.reorderFailed": { standardKey: "settings.reorderFailed", translation: "排序失败", reason: "消息" },

  // === permissions.group 组 ===
  "permissions.group.menu": { standardKey: "settings.permissions.group.menu", translation: "菜单管理", reason: "权限组" },
  "permissions.group.store": { standardKey: "settings.permissions.group.store", translation: "店铺管理", reason: "权限组" },

  // === settings.label 组 ===
  "settings.selectFieldHint": { standardKey: "settings.label.selectFieldHint", translation: "请先选择字段", reason: "提示" },
  "settings.fieldProperties": { standardKey: "settings.label.fieldProperties", translation: "字段属性", reason: "标题" },
  "settings.displayLabel": { standardKey: "settings.label.displayLabel", translation: "显示标签", reason: "字段" },
  "settings.labelPlaceholder": { standardKey: "settings.label.placeholder", translation: "请输入标签", reason: "占位符" },
  "settings.xPosition": { standardKey: "settings.label.xPosition", translation: "X 位置", reason: "字段" },
  "settings.yPosition": { standardKey: "settings.label.yPosition", translation: "Y 位置", reason: "字段" },
  "settings.fontSize": { standardKey: "settings.label.fontSize", translation: "字体大小", reason: "字段" },
  "settings.fontFamily": { standardKey: "settings.label.fontFamily", translation: "字体", reason: "字段" },
  "settings.fontStyle": { standardKey: "settings.label.fontStyle", translation: "字重", reason: "字段" },
  "settings.styleRegular": { standardKey: "settings.label.styleRegular", translation: "常规", reason: "选项" },
  "settings.styleBold": { standardKey: "settings.label.styleBold", translation: "粗体", reason: "选项" },
  "settings.textAlign": { standardKey: "settings.label.textAlign", translation: "水平对齐", reason: "字段" },
  "settings.verticalAlign": { standardKey: "settings.label.verticalAlign", translation: "垂直对齐", reason: "字段" },
  "settings.contentTemplate": { standardKey: "settings.label.contentTemplate", translation: "内容模板", reason: "字段" },
  "settings.imageTemplateHint": { standardKey: "settings.label.imageTemplateHint", translation: "使用 {字段名} 引用数据", reason: "提示" },
  "settings.separatorHint": { standardKey: "settings.label.separatorHint", translation: "分隔线用于分割标签内容", reason: "提示" },
  "settings.field.newTextField": { standardKey: "settings.label.field.newText", translation: "新建文本", reason: "操作" },
  "settings.field.defaultText": { standardKey: "settings.label.field.defaultText", translation: "默认文本", reason: "默认" },
  "settings.field.newImage": { standardKey: "settings.label.field.newImage", translation: "新建图片", reason: "操作" },
  "settings.field.defaultSeparator": { standardKey: "settings.label.field.defaultSeparator", translation: "默认分隔线", reason: "默认" },
  "settings.field.text": { standardKey: "settings.label.field.text", translation: "文本", reason: "类型" },
  "settings.field.image": { standardKey: "settings.label.field.image", translation: "图片", reason: "类型" },
  "settings.field.line": { standardKey: "settings.label.field.line", translation: "线条", reason: "类型" },
  "settings.selectPrinterFirst": { standardKey: "settings.label.selectPrinterFirst", translation: "请先选择打印机", reason: "提示" },
  "settings.invalidJson": { standardKey: "settings.label.invalidJson", translation: "JSON 格式错误", reason: "验证" },
  "settings.testSent": { standardKey: "settings.label.testSent", translation: "测试标签已发送", reason: "消息" },
  "settings.testFailed": { standardKey: "settings.label.testFailed", translation: "测试失败", reason: "消息" },
  "settings.horizontalLine": { standardKey: "settings.label.horizontalLine", translation: "水平线", reason: "元素" },
  "settings.noLayers": { standardKey: "settings.label.noLayers", translation: "暂无图层", reason: "空状态" },
  "settings.templateName": { standardKey: "settings.label.templateName", translation: "模板名称", reason: "表单" },
  "settings.widthMm": { standardKey: "settings.label.widthMm", translation: "宽度 (mm)", reason: "表单" },
  "settings.heightMm": { standardKey: "settings.label.heightMm", translation: "高度 (mm)", reason: "表单" },
  "settings.paddingX": { standardKey: "settings.label.paddingX", translation: "水平边距", reason: "表单" },
  "settings.paddingY": { standardKey: "settings.label.paddingY", translation: "垂直边距", reason: "表单" },
  "settings.showOffsetBorder": { standardKey: "settings.label.showOffsetBorder", translation: "显示偏移边框", reason: "选项" },
  "settings.renderDpi": { standardKey: "settings.label.renderDpi", translation: "渲染 DPI", reason: "表单" },
  "settings.renderDpiHint": { standardKey: "settings.label.renderDpiHint", translation: "通常 203 或 300", reason: "提示" },
  "settings.selectElementHint": { standardKey: "settings.label.selectElementHint", translation: "点击选择元素", reason: "提示" },
  "settings.testDataJson": { standardKey: "settings.label.testDataJson", translation: "测试数据 (JSON)", reason: "表单" },
  "settings.fillSample": { standardKey: "settings.label.fillSample", translation: "填充示例数据", reason: "操作" },
  "settings.testDataHint": { standardKey: "settings.label.testDataHint", translation: "使用 {字段名} 引用", reason: "提示" },
  "settings.zoomIn": { standardKey: "settings.label.zoomIn", translation: "放大", reason: "操作" },
  "settings.zoomOut": { standardKey: "settings.label.zoomOut", translation: "缩小", reason: "操作" },
  "settings.fitToScreen": { standardKey: "settings.label.fitToScreen", translation: "适应屏幕", reason: "操作" },

  // === settings.fieldHelper ===
  "settings.fieldKey": { standardKey: "settings.label.fieldKey", translation: "字段键", reason: "帮助" },
  "settings.common.description": { standardKey: "settings.label.description", translation: "描述", reason: "帮助" },
  "settings.common.example": { standardKey: "settings.label.example", translation: "示例", reason: "帮助" },
  "settings.fieldHelperHint": { standardKey: "settings.label.fieldHelperHint", translation: "支持字段引用", reason: "提示" },

  // === settings.productOrder ===
  "settings.productOrder": { standardKey: "settings.productOrder", translation: "商品排序", reason: "功能" },
  "settings.dragToReorder": { standardKey: "settings.dragToReorder", translation: "拖拽排序", reason: "提示" },
  "settings.noProducts": { standardKey: "settings.product.noProducts", translation: "暂无商品", reason: "空状态" },

  // === settings.specificationManagement ===
  "settings.manageSpecifications": { standardKey: "settings.specification.manage", translation: "管理规格", reason: "操作" },
  "settings.multiSpecDisabled": { standardKey: "settings.specification.multiDisabled", translation: "多规格功能未启用", reason: "状态" },
  "settings.enableMultiSpecHint": { standardKey: "settings.specification.enableMultiHint", translation: "启用后可添加多个规格", reason: "提示" },
  "settings.enableMultiSpec": { standardKey: "settings.specification.enableMulti", translation: "启用多规格", reason: "选项" },
  "settings.specificationNameRequired": { standardKey: "settings.specification.form.nameRequired", translation: "请输入规格名称", reason: "验证" },
  "settings.specificationUpdated": { standardKey: "settings.specification.message.updated", translation: "规格已更新", reason: "消息" },
  "settings.specificationCreated": { standardKey: "settings.specification.message.created", translation: "规格已创建", reason: "消息" },
  "settings.specificationDeleted": { standardKey: "settings.specification.message.deleted", translation: "规格已删除", reason: "消息" },

  // === statistics.status ===
  "statistics.status.completed": { standardKey: "statistics.status.completed", translation: "已完成", reason: "订单状态" },
  "statistics.status.voided": { standardKey: "statistics.status.voided", translation: "已作废", reason: "订单状态" },
  "statistics.status.merged": { standardKey: "statistics.status.merged", translation: "已合并", reason: "订单状态" },
};

function addNestedKey(obj: Record<string, any>, path: string, value: string): void {
  const parts = path.split('.');
  let current = obj;

  for (let i = 0; i < parts.length - 1; i++) {
    const part = parts[i];
    if (!(part in current) || typeof current[part] !== 'object') {
      current[part] = {};
    }
    current = current[part];
  }

  current[parts[parts.length - 1]] = value;
}

async function main() {
  const localePath = "../src/services/i18n/locales/zh-CN.json";
  console.log("🔧 修复缺失的 i18n keys...\n");

  // 读取现有 locale 文件
  const content = await Deno.readTextFile(localePath);
  const data = JSON.parse(content);

  // 统计
  let added = 0;
  let alreadyExists = 0;

  // 添加缺失的 keys
  for (const [originalKey, info] of Object.entries(missingKeys)) {
    const key = info.standardKey;
    const value = info.translation;

    // 检查是否已存在
    const parts = key.split('.');
    let current = data;
    let exists = true;
    for (const part of parts) {
      if (!(part in current) || typeof current[part] !== 'object') {
        exists = false;
        break;
      }
      current = current[part];
    }

    // 检查最后一级
    if (exists) {
      const lastKey = parts[parts.length - 1];
      if (!(lastKey in current) || current[lastKey] !== value) {
        exists = false;
      }
    }

    if (exists) {
      alreadyExists++;
      continue;
    }

    addNestedKey(data, key, value);
    added++;
  }

  // 保存文件
  const newContent = JSON.stringify(data, null, 2) + '\n';
  await Deno.writeTextFile(localePath, newContent);

  console.log("=".repeat(60));
  console.log(`\n✅ 完成!`);
  console.log(`   新增: ${added} 个 keys`);
  console.log(`   已存在: ${alreadyExists} 个 keys\n`);

  // 生成源码修改建议
  console.log("=".repeat(60));
  console.log("\n📝 建议修改源码中的 t() 调用:\n");

  const changes: Record<string, string> = {};
  for (const [originalKey, info] of Object.entries(missingKeys)) {
    if (originalKey !== info.standardKey) {
      changes[originalKey] = info.standardKey;
    }
  }

  const sortedChanges = Object.entries(changes).sort((a, b) => a[0].localeCompare(b[0]));
  for (const [oldKey, newKey] of sortedChanges) {
    console.log(`  t("${oldKey}") → t("${newKey}")`);
  }

  console.log("\n" + "=".repeat(60));
}

main();
