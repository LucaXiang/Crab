# i18n 键值命名规范

## 命名空间结构

```
namespace.subnamespace.key
```

### 一级命名空间 (必须)

| 命名空间 | 用途 | 示例 |
|---------|------|------|
| `common` | 通用UI文本、表单占位符、状态 | `common.save`, `common.namePlaceholder` |
| `app` | 应用级行为（导航、操作、退出） | `app.nav.pos`, `app.action.openCashDrawer` |
| `auth` | 认证、登录、权限、角色 | `auth.login.title`, `auth.roles.admin` |
| `pos` | POS收银界面（购物车、侧边栏） | `pos.cart.empty`, `pos.sidebar.checkout` |
| `checkout` | 结账流程 | `checkout.payment.success`, `checkout.split.title` |
| `table` | 桌台管理 | `table.selection.title`, `table.action.merge` |
| `history` | 历史订单 | `history.sidebar.title`, `history.info.table` |
| `statistics` | 数据统计 | `statistics.revenueTrend`, `statistics.topProducts` |
| `timeline` | 时间线事件 | `timeline.event.payment`, `timeline.event.splitBill` |
| `draft` | 暂存订单 | `draft.list.title`, `draft.action.restore` |
| `error` | 错误处理 | `error.network`, `error.loadFailed` |
| `settings` | 系统设置 | `settings.zone.title`, `settings.printer.form.name` |
| `fonts` | 字体名称 | `fonts.arial`, `fonts.microsoftYaHei` |

### 二级/三级命名空间 (settings 子模块)

```
settings.{module}
├── settings.product   # 商品管理
├── settings.category  # 分类管理
├── settings.attribute # 属性管理
├── settings.specification # 规格管理
├── settings.table     # 桌台管理
├── settings.zone      # 区域管理
├── settings.user      # 用户管理
├── settings.printer   # 打印机设置
├── settings.roles     # 角色权限
├── settings.system    # 系统设置
├── settings.data      # 数据管理
└── settings.common    # 设置页通用
```

## 键名命名规则

### 1. 统一使用 camelCase（驼峰命名）

```typescript
// ✅ 正确
common.saveSuccess
common.namePlaceholder
settings.product.form.name

// ❌ 错误
common.save_success
common_name_placeholder
```

### 2. 子模块内部结构统一

**表单相关**: `form.*`
```json
{
  "settings.product.form": {
    "name": "菜品名称",
    "namePlaceholder": "请输入菜品名称",
    "category": "分类",
    "selectCategory": "选择分类",
    "price": "价格",
    "externalId": "菜品编号"
  }
}
```

**操作相关**: `action.*`
```json
{
  "settings.product.action": {
    "add": "添加菜品",
    "edit": "编辑菜品",
    "delete": "删除菜品",
    "deleted": "菜品已删除"
  }
}
```

**消息提示**: `message.*` 或 `toast.*`
```json
{
  "settings.product.message": {
    "saveSuccess": "保存成功",
    "saveFailed": "保存失败",
    "deleteSuccess": "删除成功",
    "deleteFailed": "删除失败"
  }
}
```

**确认对话框**: `confirm.*`
```json
{
  "settings.product.confirm": {
    "delete": "确定要删除该菜品吗？此操作无法撤销。",
    "batchDelete": "确认批量删除"
  }
}
```

**列表/空状态**: `list.*` / `noData`
```json
{
  "settings.product.list": {
    "noData": "暂无菜品数据",
    "title": "商品列表"
  }
}
```

### 3. 状态统一使用 `status.*`

```json
{
  "common.status": {
    "enabled": "已启用",
    "disabled": "已禁用",
    "active": "激活",
    "inactive": "停用",
    "all": "全部"
  }
}
```

### 4. 通用的表单占位符

```json
{
  "common.form": {
    "name": "名称",
    "namePlaceholder": "请输入名称",
    "description": "描述",
    "descriptionPlaceholder": "请输入描述",
    "searchPlaceholder": "搜索...",
    "selectPlaceholder": "请选择"
  }
}
```

## 禁止的命名模式

```typescript
// ❌ 禁止: 下划线命名
common.save_success

// ❌ 禁止: 缩写不一致
settings.printer.kitchenPrintName  // kitchenPrint vs kitchenPrinting

// ❌ 禁止: 同一个模块混用不同风格
settings.product.action.add
settings.product.action_delete      // 混用

// ❌ 禁止: 过于通用的键名
settings.title                      // 不知道是什么的标题
common.text                         // 不知道是什么的文本
```

## 好的例子

```json
{
  "common": {
    "save": "保存",
    "cancel": "取消",
    "delete": "删除",
    "confirm": "确认",
    "form": {
      "name": "名称",
      "namePlaceholder": "请输入名称"
    },
    "status": {
      "enabled": "已启用",
      "disabled": "已禁用"
    },
    "noData": "暂无数据",
    "loading": "加载中..."
  },

  "settings": {
    "product": {
      "title": "菜品管理",
      "form": {
        "name": "菜品名称",
        "namePlaceholder": "请输入菜品名称",
        "price": "价格"
      },
      "action": {
        "add": "添加菜品",
        "edit": "编辑菜品",
        "delete": "删除菜品"
      },
      "message": {
        "saveSuccess": "保存成功",
        "deleteSuccess": "删除成功"
      },
      "confirm": {
        "delete": "确定要删除该菜品吗？"
      },
      "list": {
        "noData": "暂无菜品数据"
      }
    }
  }
}
```

## 工具

使用 `scan_missing_keys.ts` 检查缺失的 keys:
```bash
deno run --allow-read scan_missing_keys.ts
```

使用 `check_i18n_structure.ts` 检查结构问题:
```bash
deno run --allow-read check_i18n_structure.ts src/services/i18n/locales/zh-CN.json
```
