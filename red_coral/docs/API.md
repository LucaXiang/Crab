# RedCoral POS API 文档

## 概述

RedCoral POS 前端通过 HTTP RESTful API 与 Rust 后端 (crab-edge-server) 通信。

**架构**: SQL -> Rust Backend -> TypeScript Client -> React UI

## 基础信息

- **Base URL**: `http://localhost:9625` (默认开发环境)
- **认证**: Bearer Token (JWT)
- **Content-Type**: `application/json`

## 认证接口

### 登录
```http
POST /api/auth/login
Content-Type: application/json

Request:
{
  "username": "admin",
  "password": "password123"
}

Response (200):
{
  "success": true,
  "data": {
    "access_token": "eyJ...",
    "token_type": "Bearer",
    "expires_in": 86400
  }
}
```

### 获取当前用户
```http
GET /api/auth/me
Authorization: Bearer <token>

Response (200):
{
  "success": true,
  "data": {
    "id": 1,
    "username": "admin",
    "role_id": 1,
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

## 商品接口

### 获取商品列表
```http
GET /api/products?category_id=1&search=汉堡&page_size=100

Response (200):
{
  "success": true,
  "data": {
    "products": [
      {
        "id": 1,
        "uuid": "xxx",
        "name": "巨无霸汉堡",
        "receipt_name": "巨无霸",
        "price": 25.00,
        "image": null,
        "category_id": 1,
        "external_id": 1001,
        "tax_rate": 0.13,
        "sort_order": 1,
        "has_multi_spec": false,
        "is_active": true,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
      }
    ],
    "total": 10
  }
}
```

### 创建商品
```http
POST /api/products
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "新品汉堡",
  "receipt_name": "新品",
  "price": 30.00,
  "category_id": 1,
  "external_id": 1002,
  "tax_rate": 0.13
}

Response (200):
{
  "success": true,
  "data": { /* 商品对象 */ }
}
```

### 更新商品
```http
PUT /api/products/1
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "更新名称",
  "price": 35.00
}
```

### 删除商品
```http
DELETE /api/products/1
Authorization: Bearer <token>
```

### 获取商品规格
```http
GET /api/products/1/specs

Response (200):
{
  "success": true,
  "data": {
    "specs": [
      {
        "id": 1,
        "product_id": 1,
        "name": "大份",
        "receipt_name": "大",
        "price": 30.00,
        "display_order": 1,
        "is_default": true
      }
    ]
  }
}
```

### 创建商品规格
```http
POST /api/products/1/specs
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "小份",
  "price": 20.00,
  "display_order": 2,
  "is_default": false
}
```

### 获取商品属性
```http
GET /api/products/1/attributes

Response (200):
{
  "success": true,
  "data": {
    "attributes": [
      {
        "id": 1,
        "name": "口味",
        "type": "SINGLE_REQUIRED",
        "display_order": 1,
        "options": [
          {
            "id": 1,
            "name": "微辣",
            "price_modifier": 0,
            "is_default": true
          }
        ]
      }
    ]
  }
}
```

## 分类接口

### 获取分类列表
```http
GET /api/categories

Response (200):
{
  "success": true,
  "data": {
    "categories": [
      {
        "id": 1,
        "name": "汉堡",
        "kitchen_printer_id": null,
        "is_kitchen_print_enabled": false,
        "is_label_print_enabled": false,
        "sort_order": 1,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

### 创建分类
```http
POST /api/categories
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "饮料",
  "kitchen_printer_id": 1,
  "is_kitchen_print_enabled": true
}
```

### 更新分类
```http
PUT /api/categories/1
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "更新名称",
  "kitchen_printer_id": 2
}
```

### 删除分类
```http
DELETE /api/categories/1
Authorization: Bearer <token>
```

## 区域/桌台接口

### 获取区域列表
```http
GET /api/zones

Response (200):
{
  "success": true,
  "data": {
    "zones": [
      {
        "id": 1,
        "name": "大厅",
        "description": "主用餐区",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

### 创建区域
```http
POST /api/zones
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "包厢",
  "description": "VIP包厢"
}
```

### 获取桌台列表
```http
GET /api/tables

Response (200):
{
  "success": true,
  "data": {
    "tables": [
      {
        "id": 1,
        "uuid": "xxx",
        "name": "1号桌",
        "zone_id": 1,
        "capacity": 4,
        "status": "available",
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

### 创建桌台
```http
POST /api/tables
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "name": "2号桌",
  "zone_id": 1,
  "capacity": 6
}
```

## 属性模板接口

### 获取属性模板列表
```http
GET /api/attributes

Response (200):
{
  "success": true,
  "data": {
    "templates": [
      {
        "id": 1,
        "name": "口味",
        "type": "SINGLE_REQUIRED",
        "display_order": 1,
        "show_on_receipt": true,
        "receipt_name": null,
        "kitchen_printer_id": null,
        "is_global": true,
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

### 获取属性模板选项
```http
GET /api/attributes/1/options

Response (200):
{
  "success": true,
  "data": {
    "options": [
      {
        "id": 1,
        "attribute_id": 1,
        "name": "正常",
        "value_code": "normal",
        "price_modifier": 0,
        "is_default": true,
        "display_order": 1,
        "is_active": true
      }
    ]
  }
}
```

## 关联接口

### 获取分类属性关联
```http
GET /api/associations/category-attributes?category_id=1

Response (200):
{
  "success": true,
  "data": {
    "category_attributes": [
      {
        "id": 1,
        "category_id": 1,
        "attribute_id": 1,
        "is_required": true,
        "display_order": 1,
        "default_option_ids": [1]
      }
    ]
  }
}
```

### 创建分类属性关联
```http
POST /api/associations/category-attributes
Authorization: Bearer <token>
Content-Type: application/json

Request:
{
  "category_id": 1,
  "attribute_id": 2,
  "is_required": false,
  "display_order": 2,
  "default_option_ids": [1, 2]
}
```

## 标签接口

### 获取标签列表
```http
GET /api/tags

Response (200):
{
  "success": true,
  "data": {
    "tags": [
      {
        "id": 1,
        "name": "招牌",
        "color": "#FF0000",
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

## 角色权限接口

### 获取角色列表
```http
GET /api/roles

Response (200):
{
  "success": true,
  "data": {
    "roles": [
      {
        "id": 1,
        "name": "管理员",
        "description": "系统管理员",
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

### 获取角色权限
```http
GET /api/roles/1/permissions

Response (200):
{
  "success": true,
  "data": {
    "permissions": ["manage_products", "manage_orders", "view_statistics"]
  }
}
```

## 后厨打印机接口

### 获取打印机列表
```http
GET /api/kitchen/printers

Response (200):
{
  "success": true,
  "data": {
    "printers": [
      {
        "id": 1,
        "name": "后厨打印机1",
        "printer_name": "厨房打印机",
        "description": "用于打印后厨订单",
        "connection_type": "usb",
        "connection_info": "/dev/usb/lp0",
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

## 定价规则接口

### 获取定价规则列表
```http
GET /api/pricing/rules

Response (200):
{
  "success": true,
  "data": {
    "rules": [
      {
        "id": 1,
        "name": "早餐优惠",
        "type": "time_based",
        "discount_type": "percentage",
        "discount_value": 10,
        "start_time": "06:00",
        "end_time": "10:00",
        "is_active": true,
        "created_at": "2024-01-01T00:00:00Z"
      }
    ]
  }
}
```

## 健康检查接口

### 健康检查
```http
GET /health

Response (200):
{
  "status": "healthy",
  "version": "1.0.0"
}
```

### 就绪检查
```http
GET /health/ready

Response (200):
{
  "status": "ready"
}
```

### 存活检查
```http
GET /health/live

Response (200):
{
  "status": "alive"
}
```

## 前端 API 客户端使用示例

```typescript
import { createClient } from '@/infrastructure/api';

const api = createClient();

// 登录
const loginResp = await api.login({
  username: 'admin',
  password: 'password123'
});
api.setAccessToken(loginResp.data!.access_token);

// 获取商品
const productsResp = await api.listProducts({ page_size: 100 });
const products = productsResp.data?.products || [];

// 获取分类
const categoriesResp = await api.listCategories();
const categories = categoriesResp.data?.categories || [];

// 创建商品
await api.createProduct({
  name: '新品',
  price: 20.00,
  category_id: 1
});
```

## 缺失的 API (需要实现)

以下功能在前端代码中使用但后端尚未实现:

| 功能 | 前端调用 | 状态 |
|------|----------|------|
| 批量删除商品 | `api.bulkDeleteProducts(ids)` | ❌ 未实现 |
| 批量删除分类 | - | ❌ 未实现 |
| 批量更新排序 | - | ❌ 未实现 |
| 商品属性绑定 | `api.bindProductAttribute()` | ❌ 未实现 |
| 商品属性解绑 | `api.unbindProductAttribute()` | ❌ 未实现 |
| 分类属性绑定 | `api.bindCategoryAttribute()` | ❌ 未实现 |
| 分类属性解绑 | `api.unbindCategoryAttribute()` | ❌ 未实现 |
| 导入数据 | `invoke('import_data', ...)` | ❌ 未实现 |
| 导出数据 | `invoke('export_data', ...)` | ❌ 未实现 |
| 统计报表 | - | ❌ 未实现 |
| 订单管理 | - | ❌ 未实现 |
| 支付处理 | - | ❌ 未实现 |
