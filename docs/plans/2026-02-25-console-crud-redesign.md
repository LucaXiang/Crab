# Console CRUD 页面 UI/UX 重新设计

日期: 2026-02-25
状态: 设计确认

## 目标

重新设计 crab-console 的 9 个门店资源 CRUD 管理页面，实现：
1. 桌面端和手机端同等重要的响应式设计
2. 功能与 RedCoral Settings 一致
3. 全新的 UI/UX，不是复制 RedCoral

## 设计理念

Console 是老板的远程管理工具。核心原则：
- **信息密度高**：桌面端不浪费空间
- **手指友好**：手机端触控目标 ≥ 44px
- **零学习成本**：看到就会用

## 核心模式：Master-Detail 响应式

### 桌面端 (≥ 1024px)

左侧列表 (40%) + 右侧编辑面板 (60%)，同屏显示。

```
┌──────────────────────────────────────────────┐
│  FilterBar [搜索] [筛选]          [+ 新建]    │
├─────────────────────┬────────────────────────┤
│   列表 (卡片)       │   编辑面板 (inline)     │
│   ┌───────────────┐ │   ┌──────────────────┐ │
│   │ Espresso  2€ ◄│─│─►│ 商品名: Espresso │ │
│   │ Latte    3.5€ │ │   │ 价格: 2.00€      │ │
│   │ Croissant 2€  │ │   │ 分类: 饮品       │ │
│   └───────────────┘ │   │ [保存] [删除]      │ │
│                     │   └──────────────────┘ │
└─────────────────────┴────────────────────────┘
```

- 点击列表项 → 右侧面板加载该项编辑表单
- 点击 [+ 新建] → 右侧面板显示空表单
- 面板内编辑 → [保存] 提交 → 列表刷新
- 未选中任何项时，右侧显示空状态提示

### 手机端 (< 1024px)

列表全屏 → 点击推入编辑全屏页。

```
列表页:                    编辑页:
┌──────────────┐          ┌──────────────┐
│ [搜索][+新建]│          │ ← 返回       │
├──────────────┤   ──►    ├──────────────┤
│ Espresso  2€ │  点击     │ 编辑表单     │
│ Latte   3.5€ │          │ ...          │
│ Croissant 2€ │          │ [保存][删除]  │
└──────────────┘          └──────────────┘
```

- 无 Modal，全屏推入（类似 iOS 导航 push）
- 返回按钮回到列表
- 如有未保存更改，显示 "丢弃更改?" 确认

## 列表项卡片设计

不用表格，用卡片列表。卡片天然响应式，手机端不会列被挤压。

### 桌面端 (横向紧凑行)

```
┌─────────────────────────────────────────────────┐
│ ☕ Espresso          饮品  [热饮][咖啡]   2.00€ │
├─────────────────────────────────────────────────┤
│ 🥐 Matcha Croissant  烘焙  [早餐]        2.40€ │
└─────────────────────────────────────────────────┘
```

每行：图标 + 名称 + 元信息（分类、标签等） + 价格/状态 靠右。
选中项高亮（左边框色条 + 浅背景）。

### 手机端 (纵向舒展)

```
┌───────────────────────────┐
│ ☕ Espresso         2.00€ │
│ 饮品 · [热饮] [咖啡]      │
├───────────────────────────┤
│ 🥐 Matcha Croissant 2.40€│
│ 烘焙 · [早餐]             │
└───────────────────────────┘
```

两行布局：第一行 名称 + 价格，第二行 元信息标签。
触控区域整行可点击。

## 编辑面板设计

### 表单布局

```
┌──────────────────────────────────┐
│ 编辑商品                    [×]  │  ← 桌面端关闭按钮
│──────────────────────────────────│
│                                  │
│ 商品名称 *                       │
│ ┌──────────────────────────────┐ │
│ │ Espresso                     │ │
│ └──────────────────────────────┘ │
│                                  │
│ 价格 *                           │
│ ┌──────────────────────────────┐ │
│ │ 2.00                         │ │
│ └──────────────────────────────┘ │
│                                  │
│ 分类                             │
│ ┌──────────────────────────────┐ │
│ │ 饮品                      ▼ │ │
│ └──────────────────────────────┘ │
│                                  │
│ 标签                             │
│ [热饮 ×] [咖啡 ×] [+ 添加]       │
│                                  │
│ ─── 打印设置 ──────────────────  │
│ ☐ 厨房打印  ☑ 标签打印           │
│                                  │
│ ┌────────┐  ┌──────────────────┐ │
│ │  删除  │  │     保存修改      │ │
│ └────────┘  └──────────────────┘ │
└──────────────────────────────────┘
```

- 输入框高度 ≥ 44px（触控友好）
- 保存按钮始终在底部（手机端 sticky bottom）
- 删除按钮左下角，红色 outline 风格，防误触
- 表单分组用分隔线 + 小标题（如 "打印设置"）

### 新建 vs 编辑

- 新建：标题 "新建商品"，无删除按钮，保存按钮文字 "创建"
- 编辑：标题 "编辑商品"，有删除按钮，保存按钮文字 "保存修改"

## 9 个 CRUD 模块适配

| 序号 | 模块 | 卡片显示 | 编辑面板字段 | 实现优先级 |
|------|------|---------|------------|-----------|
| 1 | Zone | 名称、描述摘要 | name, description | P0 (验证模式) |
| 2 | Tag | 名称、颜色圆点 | name, color (色板选择器) | P0 |
| 3 | Table | 名称、区域名、座位数 | name, zone (select), capacity | P0 |
| 4 | Employee | 姓名、角色徽章 | name, pin, role (select) | P1 |
| 5 | Category | 名称、商品数、虚拟标记 | name, is_virtual, tags, kitchen_print, label_print | P1 |
| 6 | Product | 名称、价格、分类、标签 | name, price/specs, category, tags, tax_rate, print设置 | P2 |
| 7 | Attribute | 名称、选项数 | name + 内嵌选项列表 (子CRUD) | P2 |
| 8 | PriceRule | 名称、类型、状态徽章 | 保持现有向导，入口改为 Master-Detail 列表 | P3 |
| 9 | LabelTemplate | 名称、尺寸 | 列表用 Master-Detail，编辑器保持全屏 | P3 |

## 共享组件

### 新建

| 组件 | 文件 | 说明 |
|------|------|------|
| `MasterDetail` | `shared/components/MasterDetail.tsx` | 响应式布局容器，桌面端左右分栏，手机端列表/详情切换 |
| `ItemCard` | `shared/components/ItemCard.tsx` | 列表项卡片，支持选中高亮、状态徽章、响应式两种布局 |
| `DetailPanel` | `shared/components/DetailPanel.tsx` | 编辑面板容器，桌面端右侧 panel，手机端全屏页 |

### 保留并调整

| 组件 | 调整 |
|------|------|
| `FormField` | 输入框高度 ≥ 44px，label 字号调大 |
| `SelectField` | 下拉改为 native select（手机端体验更好） |
| `FilterBar` | 保持，调整按钮尺寸 |
| `ConfirmDialog` | 保持，用于删除确认 |
| `StatusToggle` | 保持 |
| `TagPicker` | 保持 |

### 移除

| 组件 | 原因 |
|------|------|
| `DataTable` | 被 `ItemCard` 列表替代 |

## MasterDetail 组件规格

```tsx
interface MasterDetailProps<T> {
  // 列表
  items: T[];
  renderItem: (item: T, isSelected: boolean) => React.ReactNode;
  selectedId: string | number | null;
  onSelect: (item: T) => void;

  // 搜索
  searchQuery: string;
  onSearchChange: (query: string) => void;
  totalCount: number;

  // 新建
  onCreateNew: () => void;
  createLabel: string;

  // 编辑面板
  children: React.ReactNode; // DetailPanel 内容
  isCreating: boolean;       // 新建模式

  // 主题
  themeColor?: 'blue' | 'teal' | 'orange' | 'purple' | 'indigo';
}
```

### 响应式行为

```tsx
// 桌面端 (≥ lg)
<div className="flex h-full">
  <div className="w-[40%] border-r overflow-y-auto">
    {/* 搜索栏 + 列表 */}
  </div>
  <div className="flex-1 overflow-y-auto">
    {selectedId || isCreating ? children : <EmptyState />}
  </div>
</div>

// 手机端 (< lg)
{showDetail ? (
  <div className="fixed inset-0 z-40 bg-white">
    <header>← 返回</header>
    {children}
  </div>
) : (
  <div className="h-full overflow-y-auto">
    {/* 搜索栏 + 列表 */}
  </div>
)}
```

## 不改动的部分

- 路由结构（保持独立路由 /stores/:id/products 等）
- StoreLayout 导航侧栏和移动端 tab
- API 层 (infrastructure/api/)
- 实时订单、日报、统计等非 CRUD 页面
- PriceRule 向导内部
- LabelTemplate 编辑器内部

## 实现顺序

### Phase 1: 基础设施 + 验证

1. 创建 `MasterDetail` 组件
2. 创建 `ItemCard` 组件
3. 创建 `DetailPanel` 组件
4. 用 Zone 模块验证完整流程（最简单的 CRUD）

### Phase 2: 简单模块

5. Tag 管理（加颜色选择器）
6. Table 管理（加区域关联）
7. Employee 管理（加角色选择 + PIN）

### Phase 3: 中等模块

8. Category 管理（虚拟分类 tab + 标签 + 打印设置）
9. Product 管理（SKU 规格 + 多标签 + 打印设置）

### Phase 4: 复杂模块

10. Attribute 管理（主从两级，选项内嵌编辑）
11. PriceRule 列表页改为 Master-Detail（向导不变）
12. LabelTemplate 列表页改为 Master-Detail（编辑器不变）
