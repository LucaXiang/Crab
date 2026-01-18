# 前端 Store 迁移示例

## 示例：将 useAuthStore 从 Tauri commands 迁移到 HTTP API

### 更新前 (旧实现 - Tauri commands)

```typescript
// src/core/stores/auth/useAuthStore.ts (旧版)
import { invoke } from '@tauri-apps/api/core';
import type { User } from '@/core/domain/types';

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isAuthenticated: false,
  isLoading: false,

  login: async (username: string, password: string) => {
    set({ isLoading: true });
    try {
      const result = await invoke<{ user: User; token: string }>('authenticate_user', {
        username,
        password,
      });
      set({
        user: result.user,
        isAuthenticated: true,
        isLoading: false,
      });
      localStorage.setItem('token', result.token);
      return result;
    } catch (error) {
      set({ isLoading: false });
      throw error;
    }
  },

  logout: async () => {
    await invoke('logout');
    set({ user: null, isAuthenticated: false });
    localStorage.removeItem('token');
  },

  fetchCurrentUser: async () => {
    try {
      const user = await invoke<User>('get_current_user');
      set({ user, isAuthenticated: true });
    } catch (error) {
      set({ user: null, isAuthenticated: false });
    }
  },
}));
```

### 更新后 (新实现 - HTTP API)

```typescript
// src/core/stores/auth/useAuthStore.ts (新版)
import { create } from 'zustand';
import { createClient, type LoginRequest, type CurrentUser } from '@/infrastructure/api';
import type { User } from '@/core/domain/types';

const api = createClient();

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

export const useAuthStore = create<AuthState & {
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  fetchCurrentUser: () => Promise<void>;
  changePassword: (oldPassword: string, newPassword: string) => Promise<void>;
}>((set) => ({
  user: null,
  isAuthenticated: false,
  isLoading: false,
  error: null,

  login: async (username: string, password: string) => {
    set({ isLoading: true, error: null });
    try {
      const request: LoginRequest = { username, password };
      const response = await api.login(request);

      if (response.data) {
        const { access_token, user: userData } = response.data;

        // 将 API 用户数据转换为本地 User 类型
        const user: User = {
          id: userData.id,
          username: userData.username,
          displayName: userData.display_name,
          role: userData.role_name,
          // ... 其他字段映射
        };

        // 设置访问令牌
        api.setAccessToken(access_token);

        set({
          user,
          isAuthenticated: true,
          isLoading: false,
          error: null,
        });

        // 保存到 localStorage
        localStorage.setItem('access_token', access_token);
      }
    } catch (error: any) {
      set({
        isLoading: false,
        error: error.message || '登录失败',
      });
      throw error;
    }
  },

  logout: () => {
    api.clearAccessToken();
    localStorage.removeItem('access_token');
    set({
      user: null,
      isAuthenticated: false,
      error: null,
    });
  },

  fetchCurrentUser: async () => {
    try {
      const response = await api.getCurrentUser();

      if (response.data?.user) {
        const userData = response.data.user;
        const user: User = {
          id: userData.id,
          username: userData.username,
          displayName: userData.display_name,
          role: userData.role_name,
          permissions: userData.permissions,
          // ... 其他字段映射
        };

        set({
          user,
          isAuthenticated: true,
        });
      }
    } catch (error) {
      set({
        user: null,
        isAuthenticated: false,
      });
    }
  },

  changePassword: async (oldPassword: string, newPassword: string) => {
    set({ isLoading: true, error: null });
    try {
      await api.changePassword({
        old_password: oldPassword,
        new_password: newPassword,
      });
      set({ isLoading: false });
    } catch (error: any) {
      set({
        isLoading: false,
        error: error.message || '修改密码失败',
      });
      throw error;
    }
  },
}));

// 在应用启动时恢复会话
export const initializeAuth = async () => {
  const token = localStorage.getItem('access_token');
  if (token) {
    api.setAccessToken(token);
    const store = useAuthStore.getState();
    await store.fetchCurrentUser();
  }
};
```

## 关键变化说明

### 1. 导入变更

**旧:**
```typescript
import { invoke } from '@tauri-apps/api/core';
```

**新:**
```typescript
import { createClient } from '@/infrastructure/api';
```

### 2. API 调用变更

**旧 (Tauri):**
```typescript
const result = await invoke<T>('command_name', { param: value });
```

**新 (HTTP):**
```typescript
const response = await api.methodName(params);
// 访问: response.data
```

### 3. 认证处理

**旧:**
```typescript
// 令牌通过 Tauri 自动处理
```

**新:**
```typescript
// 手动管理 Bearer Token
api.setAccessToken(token);
localStorage.setItem('token', token);
```

### 4. 错误处理

**旧:**
```typescript
try {
  await invoke(...);
} catch (error) {
  // Tauri 错误
}
```

**新:**
```typescript
try {
  const response = await api.login(...);
  // 检查 response.error_code
} catch (error: any) {
  // HTTP 错误 + API 错误
  console.error(error.code, error.message);
}
```

## 完整迁移检查清单

### ✅ 需要更新的文件

1. **Stores:**
   - [ ] `src/core/stores/auth/useAuthStore.ts`
   - [ ] `src/core/stores/product/useProductStore.ts`
   - [ ] `src/core/stores/category/useCategoryStore.ts`
   - [ ] `src/core/stores/order/useOrderStore.ts`
   - [ ] `src/core/stores/table/useTableStore.ts`
   - [ ] 其他 stores...

2. **Services:**
   - [ ] `src/core/services/order/orderService.ts`
   - [ ] `src/services/printService.ts`
   - [ ] 其他 services...

3. **Components:**
   - [ ] 更新所有使用旧类型的组件
   - [ ] 更新所有调用 stores 的组件

### 📝 迁移步骤

1. **安装依赖** (已完成的)
   ```bash
   # 无需额外安装，API 客户端使用原生 fetch
   ```

2. **更新 Store**
   - 替换 invoke 调用为 API 客户端调用
   - 更新认证令牌管理
   - 更新错误处理

3. **更新 Service 层**
   - 替换直接的 Tauri 调用
   - 使用 stores 代替直接 API 调用

4. **更新组件**
   - 更新类型导入
   - 确保使用最新的 store 方法

5. **测试**
   - 验证每个功能模块
   - 检查认证流程
   - 测试错误处理

## 常用 API 客户端方法对照表

| 功能 | Tauri Command | HTTP API |
|------|---------------|----------|
| 登录 | `authenticate_user` | `api.login(data)` |
| 获取用户 | `get_current_user` | `api.getCurrentUser()` |
| 获取产品列表 | `fetch_products` | `api.listProducts(params)` |
| 创建产品 | `create_product` | `api.createProduct(data)` |
| 更新产品 | `update_product` | `api.updateProduct(id, data)` |
| 删除产品 | `delete_product` | `api.deleteProduct(id)` |
| 获取分类 | `fetch_categories` | `api.listCategories()` |

更多方法请参考：`src/infrastructure/api/client.ts`
