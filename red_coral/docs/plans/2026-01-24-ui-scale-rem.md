# UI 缩放功能设计

## 概述

收银系统运行在不同尺寸设备上，需要让用户能够自定义界面缩放。

## 技术方案

使用 CSS rem 单位 + html font-size 实现全局缩放。

```css
:root {
  --ui-scale: 1;
}

html {
  font-size: calc(16px * var(--ui-scale));
}
```

## 参数

- 范围: 90% ~ 130%
- 步进: 5%
- 默认: 100%

## 数据流

```
用户调节滑块
  → Zustand store (scale: number)
  → localStorage 持久化
  → CSS 变量 document.documentElement.style.setProperty('--ui-scale', value)
  → 所有 rem 单位自动响应
```

## 实施任务

### 任务 1: 基础设施
- [ ] index.css 添加 CSS 变量和 html font-size 规则
- [ ] 创建 useUIScaleStore.ts
- [ ] App 初始化时加载缩放设置

### 任务 2: 设置页面 UI
- [ ] SystemSettings.tsx 添加缩放滑块
- [ ] 实时预览效果
- [ ] 恢复默认按钮

### 任务 3: 清理 px 硬编码
- [ ] 将 `w-[xxxpx]` 转为 `w-[xxxrem]` 或 Tailwind 标准类
- [ ] 将 `h-[xxxpx]` 转为 `h-[xxxrem]` 或 Tailwind 标准类
- [ ] 将 `text-[xxxpx]` 转为 `text-[xxxrem]` 或 Tailwind 标准类
- [ ] 保留边框等不需要缩放的 px

## 例外情况（保留 px）

- `border-[1px]` - 边框保持锐利
- 阴影相关值
- 特殊对齐需求
