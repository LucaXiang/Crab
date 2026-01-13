# Tauri 架构演进：告别 Command 样板代码

## 1. 问题现状：Command 爆炸

在传统的 Tauri 开发模式中，随着业务增长，我们容易陷入 "Command Hell"：

```rust
// 典型的 "坏味道" 代码
#[tauri::command]
fn create_order(state: State<App>, item_id: String, qty: u32) -> Result<(), String> { ... }

#[tauri::command]
fn cancel_order(state: State<App>, order_id: String) -> Result<(), String> { ... }

#[tauri::command]
fn add_discount(state: State<App>, order_id: String, discount: f64) -> Result<(), String> { ... }

// ... 几十个类似的函数 ...

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            create_order,
            cancel_order,
            add_discount,
            // ... 几十个函数名 ...
        ])
        // ...
}
```

**痛点：**
1.  **样板代码多**：每个操作都要写一个 wrapper 函数，然后在 `generate_handler!` 里注册。
2.  **类型重复**：前端要手动定义参数类型，或者依赖不够灵活的自动生成工具。
3.  **维护困难**：修改一个参数需要在 Rust Command 定义、Rust 业务逻辑、Frontend 调用处同时修改。

## 2. 解决方案：基于 Intent 的单通道分发 (Unified Intent Dispatch)

既然我们在 `shared` crate 中已经有了能够表达所有业务意图的 `OrderIntent` 枚举，我们可以直接利用它。

### 2.1 核心思想
不再为每个动作写 Command，而是只暴露**一个**万能的 Command：`dispatch`。

前端发送一个序列化的 `OrderIntent`，后端反序列化后直接丢给业务层处理。

### 2.2 Rust 端实现

```rust
// src-tauri/src/lib.rs

use shared::message::OrderIntent;
use crab_client::CrabClient; // 假设我们复用了 client 逻辑

// 唯一的 Command
#[tauri::command]
async fn dispatch(
    state: tauri::State<'_, Arc<dyn CrabClient>>, 
    intent: OrderIntent
) -> Result<(), String> {
    // 直接转发给核心业务逻辑
    state.post_intent(intent).await
        .map_err(|e| e.to_string())
}

// 注册时极其清爽
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![dispatch])
        // ...
}
```

### 2.3 前端调用 (TypeScript)

配合 `ts-rs` 或 `specta`，我们可以自动从 Rust 生成 TypeScript 类型。

```typescript
// types/generated.ts (自动生成)
export type OrderIntent = 
  | { type: "CreateOrder", table_id: string, items: Item[] }
  | { type: "AddItem", order_id: string, item_id: string }
  | { type: "Pay", order_id: string, amount: number };

// api.ts
import { invoke } from "@tauri-apps/api/core";

export async function dispatch(intent: OrderIntent) {
    return invoke("dispatch", { intent });
}

// 使用
await dispatch({ 
    type: "CreateOrder", 
    table_id: "A1", 
    items: [] 
});
```

## 3. 通用 CRUD 设计：管理后台的救星

对于“添加菜品”、“修改价格”、“打标签”这类管理操作，逻辑与订单完全一致，只是数据模型不同。

### 3.1 写操作 (CUD)：引入 DataIntent

我们不需要为 `add_dish`, `update_dish`, `delete_dish` 写三个 command。我们只需要定义一个 `DataIntent`。

在 `shared` 中定义：

```rust
// shared/src/message/payload.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "model", content = "action")] 
// 前端 JSON 看起来像: { "model": "Dish", "action": { "type": "Create", "data": {...} } }
pub enum DataIntent {
    Dish(CrudAction<DishData>),
    Category(CrudAction<CategoryData>),
    Tag(CrudAction<TagData>),
    // 未来添加 User, Printer, Table 等等...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CrudAction<T> {
    Create(T),
    Update { id: String, diff: Partial<T> }, // Partial<T> 可以是 Option 字段的结构体
    Delete { id: String },
}
```

Tauri 侧依然只需要**一个**接口：

```rust
#[tauri::command]
async fn dispatch_data(
    state: State<'_, Arc<dyn DataService>>, 
    intent: DataIntent
) -> Result<(), String> {
    state.apply(intent).await
}
```

前端调用：

```typescript
// 添加菜品
await dispatchData({
  model: "Dish",
  action: {
    type: "Create",
    data: { name: "宫保鸡丁", price: 3800, category: "hot_dish" }
  }
});

// 修改菜品
await dispatchData({
  model: "Dish",
  action: {
    type: "Update",
    data: { id: "dish_123", diff: { price: 4200 } }
  }
});
```

### 3.2 读操作 (Read)：统一查询接口

读操作通常比写操作更灵活（分页、过滤、排序）。我们可以定义一个通用的 `Query` 结构。

```rust
// shared/src/query.rs

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub model: String,       // "Dish", "Order", "Category"
    pub filter: Option<Value>, // { "category": "hot_dish", "price_gt": 1000 }
    pub sort: Option<String>,  // "price_desc"
    pub page: Option<u32>,
    pub limit: Option<u32>,
}
```

Tauri 接口：

```rust
#[tauri::command]
async fn query_list(
    state: State<'_, Arc<dyn QueryService>>, 
    request: QueryRequest
) -> Result<PaginatedResponse<Value>, String> {
    state.find(request).await
}

#[tauri::command]
async fn query_one(
    state: State<'_, Arc<dyn QueryService>>, 
    model: String,
    id: String
) -> Result<Option<Value>, String> {
    state.find_by_id(model, id).await
}
```

前端调用：

```typescript
// 获取所有热菜，按价格排序
const dishes = await queryList({
  model: "Dish",
  filter: { category: "hot_dish" },
  sort: "price_desc"
});
```

### 3.3 总结：两套 API 走天下

通过这种设计，无论你的系统有多少个数据模型（菜品、员工、打印机、会员...），你的 Tauri 接口永远只有这几个：

1.  `dispatch_order(intent)`: 处理高并发、即时性强的订单业务。
2.  `dispatch_data(intent)`: 处理低频、CRUD 为主的管理业务。
3.  `query_list(req)`: 通用列表查询。
4.  `query_one(model, id)`: 通用详情查询。

**不仅代码优雅，而且：**
*   **前端开发极快**：不需要等后端写接口，只要数据模型定了，直接调通用接口。
*   **后端极其稳定**：新增一个 "会员" 模块，只需要在 `DataIntent` 里加一行枚举，实现对应的 Service 逻辑，**Tauri 层代码一行都不用改**。

## 5. 拒绝 "Switch 地狱"：像写 Axum 一样写 Tauri

你担心的完全正确：如果 `dispatch_data` 里写几百行 `match`，那确实是噩梦。

我也非常理解你想要 "点对点" (Point-to-Point) 的感觉——即“一个请求直达一个处理函数”，而不是经过一堆乱七八糟的路由逻辑。

我们可以利用 Rust 的 **泛型 Trait** 和 **宏**，实现类似 Web 框架（如 Axum）的开发体验，同时保持 Tauri 通道的单一性（为了避开 HTTPS 证书问题）。

### 5.1 核心：Handler Trait

我们定义一个 `Handler` trait，任何 Service 只要实现了它，就能自动处理对应的 Intent。

```rust
// 后端：定义处理规范
#[async_trait]
pub trait IntentHandler<T>: Send + Sync {
    async fn handle(&self, action: CrudAction<T>) -> Result<(), String>;
}
```

### 5.2 业务层实现 (像写 Controller 一样)

```rust
// DishService.rs - 只需要关注 Dish 逻辑
#[async_trait]
impl IntentHandler<DishData> for DishService {
    async fn handle(&self, action: CrudAction<DishData>) -> Result<(), String> {
        match action {
            CrudAction::Create(data) => self.repo.create(data).await,
            CrudAction::Update { id, diff } => self.repo.update(id, diff).await,
            CrudAction::Delete { id } => self.repo.delete(id).await,
        }
    }
}
```

### 5.3 自动分发层 (Router)

我们可以用一个简单的宏来消除 `main` 函数里的 switch。

```rust
// 这是一个伪代码宏示例，实际可以用声明式宏实现
// 只要注册了 Service，宏自动生成 match 分支
router!(intent, state, [
    (Model::Dish, state.dish_service),
    (Model::User, state.user_service),
    (Model::Printer, state.printer_service),
]);
```

这样，你的 `dispatch_data` 函数永远只有几行，业务逻辑全部分散在各个 Service 里，**实现了逻辑上的“点对点”**。

## 6. 为什么不能直接用 HTTPS (Axum)？

你提到的 *"Tauri 前端不信任 HTTPS"* 是一个极其痛点的问题，也是我们坚持用 Tauri Command (IPC) 的核心原因。

### 6.1 浏览器的安全限制
Tauri 的前端本质上是一个 Webview (Safari/WebKit)。现代 Webview 对安全性要求极高：
1.  **混合内容 (Mixed Content)**: 如果你的前端运行在 `tauri://localhost` (被视为安全上下文)，它**禁止**请求不安全的 HTTP 接口。
2.  **自签名证书 (Self-Signed Certs)**: 如果你给本地 Axum 上了 HTTPS（用自签名证书），Webview 会直接拦截请求（`ERR_CERT_AUTHORITY_INVALID`），并且**通常没有 UI 让用户点击“继续访问”**。

### 6.2 IPC 通道作为“安全隧道”
Tauri 的 `invoke` 命令是通过操作系统底层的 IPC 管道（Pipe/Socket）传输的，**不走网络协议栈**。

*   **没有 SSL 握手**：完全避开了证书验证问题。
*   **没有 CORS**：不需要处理跨域。
*   **极致性能**：数据在内存中传递，比 HTTP 解析更快。

### 6.3 最终形态：伪装成 HTTP 的 IPC

为了让你写前端时感觉像是在调 HTTP 接口，我们可以封装一层 SDK：

```typescript
// frontend/api/dish.ts
// 看起来完全像是一个 HTTP 请求库
export const DishApi = {
    create: (data) => request('Dish', 'Create', data),
    update: (id, data) => request('Dish', 'Update', { id, diff: data }),
    delete: (id) => request('Dish', 'Delete', { id }),
};

// 底层实现
function request(model, type, data) {
    // 可以在这里加日志、拦截器，就像 Axios 一样
    console.log(`[API] ${model}.${type}`, data);
    return tauri.invoke('dispatch_data', { 
        intent: { model, action: { type, data } } 
    });
}
```

这样，你的开发体验是：
1.  **后端**：写独立的 Service 实现 `handle` 方法（逻辑解耦）。
2.  **前端**：调 `DishApi.create()`（像 REST API）。
3.  **中间**：Tauri IPC 默默地充当了那条“可信的隧道”。

## 8. 回归朴素：显式代码生成 (Explicit Codegen)

我理解你的感受。用 `Proxy` 搞“运行时黑魔法”确实让人心里没底：
1.  **不透明**：出了问题不知道是 Proxy 写错了还是传参传错了。
2.  **不可见**：`api.dish` 在代码里根本找不到定义，Ctrl+Click 跳不过去。
3.  **调试难**：在 Chrome DevTools 里看到的调用栈是一堆 Proxy Handler。

我们不妨返璞归真，用 **代码生成 (Code Generation)** 来替代运行时魔法。这是最稳健、最符合直觉的做法（类似 gRPC / Thrift 的思路）。

### 8.1 目标效果：看得见摸得着的代码

我们希望有一个脚本，自动扫描 Rust 代码，生成**真实存在**的 TypeScript 文件：

```typescript
// frontend/src/api/generated/dish.ts
// ⚠️ 此文件由脚本自动生成，请勿手动修改

import { invoke } from "@tauri-apps/api/core";
import type { Dish, CreateDishDto, UpdateDishDto } from "../types";

export const DishApi = {
    create: async (data: CreateDishDto) => {
        return invoke("dispatch_data", { 
            intent: { model: "Dish", action: { type: "Create", data } } 
        });
    },
    
    update: async (id: string, diff: UpdateDishDto) => {
        return invoke("dispatch_data", { 
            intent: { model: "Dish", action: { type: "Update", id, diff } } 
        });
    },
    
    delete: async (id: string) => {
        return invoke("dispatch_data", { 
            intent: { model: "Dish", action: { type: "Delete", id } } 
        });
    }
};
```

### 8.2 为什么这样更好？

1.  **点对点 (Point-to-Point)**：你在业务代码里调用 `DishApi.create(...)`，这就是一个普通的函数调用，没有任何魔法。
2.  **IDE 友好**：你可以 Ctrl+Click 跳转到定义，看到它到底发了什么。
3.  **编译器保障**：如果 Rust 端的 `Dish` 模型改了名字，重新生成 TS 后，所有调用的地方都会报错，编译都通不过（这是好事！）。

### 8.3 如何实现？

我们不需要引入复杂的框架，写一个简单的 Rust `build.rs` 或者独立的 `codegen` bin 工具即可。

#### 步骤 1: 标记模型
在 Rust 里，我们用一个 Trait 或宏标记哪些 Struct 需要生成 API。

```rust
// shared/src/models.rs
#[derive(TS, Serialize, Deserialize)] // 使用 ts-rs 生成类型
#[api_model] // 我们自定义的宏，标记需要生成 CRUD API
pub struct Dish { ... }
```

#### 步骤 2: 生成脚本 (Codegen Script)
写一个简单的 Rust 脚本，遍历这些 Struct，利用字符串模板生成 `.ts` 文件。

```rust
// 伪代码思路
fn main() {
    let models = vec!["Dish", "User", "Printer"];
    
    for model in models {
        let ts_code = format!(r#"
            export const {0}Api = {{
                create: (data) => invoke("dispatch", {{ model: "{0}", ... }}),
                // ...
            }};
        "#, model);
        
        fs::write(format!("frontend/api/{}.ts", model.to_lowercase()), ts_code);
    }
}
```

### 8.4 总结

这种方案结合了 **"Unified Dispatch" (后端的简洁)** 和 **"Explicit SDK" (前端的直观)**。

*   **后端**：依然只有一个 `dispatch` 接口，不用写几百个 Command。
*   **前端**：依然拥有几百个清晰的 API 函数，但全是自动生成的。
*   **你**：只需要维护 Rust 里的 Struct 定义，剩下的全自动。

