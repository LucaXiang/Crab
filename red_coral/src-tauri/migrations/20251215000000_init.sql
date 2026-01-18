-- =============================================================================
-- RedCoral POS 数据库初始化迁移
-- =============================================================================
-- 设计原则:
-- 1. 所有金额字段以"分"为单位存储，使用 INTEGER 类型，避免浮点精度问题
-- 2. 主键统一使用 INTEGER AUTOINCREMENT，业务表增加 uuid 字段供外部系统对接
-- 3. products.id 保留 TEXT 格式（配合 external_id 使用）
-- 4. 软删除机制: is_deleted 标记 + deleted_at + deleted_by
-- 5. 订单和订单事件只读，任何操作通过新增事件记录
-- 6. audit_log 表记录系统操作，支持税务审计追溯
-- 7. 哈希链保护订单和事件防篡改
-- =============================================================================

-- =============================================================================
-- 1. 用户权限系统
-- =============================================================================

-- 角色表
CREATE TABLE IF NOT EXISTS roles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT,
    is_system INTEGER DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT
);

-- 角色权限关联表
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id INTEGER NOT NULL,
    permission TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (role_id, permission),
    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
);

-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT NOT NULL,
    role_id INTEGER NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_login INTEGER,
    avatar TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);

-- =============================================================================
-- 2. 打印机配置
-- =============================================================================

-- 厨房打印机表
CREATE TABLE IF NOT EXISTS kitchen_printers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    printer_name TEXT,
    description TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT
);

-- =============================================================================
-- 3. 产品分类
-- =============================================================================

-- 分类表
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL UNIQUE,
    sort_order INTEGER DEFAULT 0,
    kitchen_printer_id INTEGER,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT 1,
    is_label_print_enabled INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(kitchen_printer_id) REFERENCES kitchen_printers(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name);

-- =============================================================================
-- 4. 产品
-- =============================================================================

-- 产品表
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    image TEXT NOT NULL,
    category_id INTEGER NOT NULL,
    sort_order INTEGER DEFAULT 0,
    tax_rate REAL NOT NULL DEFAULT 0.10,
    has_multi_spec INTEGER NOT NULL DEFAULT 0,
    receipt_name TEXT,
    kitchen_print_name TEXT,
    kitchen_printer_id INTEGER,
    is_kitchen_print_enabled INTEGER NOT NULL DEFAULT -1,
    is_label_print_enabled INTEGER NOT NULL DEFAULT -1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(category_id) REFERENCES categories(id)
);

CREATE INDEX IF NOT EXISTS idx_products_category ON products(category_id);

-- =============================================================================
-- 5. 产品规格
-- =============================================================================

-- 产品规格表
CREATE TABLE IF NOT EXISTS product_specifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    price INTEGER NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_root INTEGER NOT NULL DEFAULT 0,
    external_id INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE CASCADE,
    UNIQUE(product_id, name),
    CHECK(is_default IN (0, 1)),
    CHECK(is_active IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_specs_product ON product_specifications(product_id);
CREATE INDEX IF NOT EXISTS idx_specs_active ON product_specifications(product_id, is_active, display_order) WHERE is_active = 1;

-- =============================================================================
-- 6. 标签系统
-- =============================================================================

-- 标签表
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    display_order INTEGER DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_tags_display_order ON tags(display_order);

-- 规格标签关联表
CREATE TABLE IF NOT EXISTS specification_tags (
    specification_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    PRIMARY KEY (specification_id, tag_id),
    FOREIGN KEY(specification_id) REFERENCES product_specifications(id) ON DELETE CASCADE,
    FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- =============================================================================
-- 7. 属性系统
-- =============================================================================

-- 属性模板表
CREATE TABLE IF NOT EXISTS attribute_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    display_order INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,
    show_on_receipt INTEGER NOT NULL DEFAULT 0,
    receipt_name TEXT,
    kitchen_printer_id INTEGER,
    is_global INTEGER DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(kitchen_printer_id) REFERENCES kitchen_printers(id) ON DELETE SET NULL
);

-- 属性选项表
CREATE TABLE IF NOT EXISTS attribute_options (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    attribute_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    value_code TEXT,
    price_modifier INTEGER DEFAULT 0,
    is_default INTEGER DEFAULT 0,
    display_order INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,
    receipt_name TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(attribute_id) REFERENCES attribute_templates(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_attr_options_attr ON attribute_options(attribute_id);

-- 产品属性绑定表
CREATE TABLE IF NOT EXISTS product_attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    product_id TEXT NOT NULL,
    attribute_id INTEGER NOT NULL,
    is_required INTEGER DEFAULT 0,
    display_order INTEGER DEFAULT 0,
    default_option_id TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(product_id) REFERENCES products(id) ON DELETE CASCADE,
    FOREIGN KEY(attribute_id) REFERENCES attribute_templates(id) ON DELETE CASCADE,
    UNIQUE(product_id, attribute_id)
);

CREATE INDEX IF NOT EXISTS idx_prod_attrs_product ON product_attributes(product_id);
CREATE INDEX IF NOT EXISTS idx_prod_attrs_attr ON product_attributes(attribute_id);

-- 分类属性绑定表
CREATE TABLE IF NOT EXISTS category_attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL,
    attribute_id INTEGER NOT NULL,
    is_required INTEGER DEFAULT 0,
    display_order INTEGER DEFAULT 0,
    default_option_id TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY(attribute_id) REFERENCES attribute_templates(id) ON DELETE CASCADE,
    UNIQUE(category_id, attribute_id)
);

-- =============================================================================
-- 8. 区域和桌台
-- =============================================================================

-- 区域表
CREATE TABLE IF NOT EXISTS zones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_zones_name ON zones(name);
CREATE INDEX IF NOT EXISTS idx_zones_active ON zones(is_active);

-- 桌台表
CREATE TABLE IF NOT EXISTS tables (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    zone_id INTEGER NOT NULL,
    capacity INTEGER NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    FOREIGN KEY(zone_id) REFERENCES zones(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tables_zone ON tables(zone_id);

-- =============================================================================
-- 9. 价格调整规则
-- =============================================================================

CREATE TABLE IF NOT EXISTS price_adjustment_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    receipt_name TEXT NOT NULL,
    description TEXT,
    rule_type TEXT NOT NULL,
    scope TEXT NOT NULL,
    target_id TEXT,
    zone_id INTEGER,
    adjustment_type TEXT NOT NULL,
    adjustment_value REAL NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    is_stackable INTEGER NOT NULL DEFAULT 1,
    time_mode TEXT NOT NULL DEFAULT 'ALWAYS',
    start_time INTEGER,
    end_time INTEGER,
    schedule_config_json TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_by TEXT,
    created_by TEXT,
    FOREIGN KEY(zone_id) REFERENCES zones(id),
    CHECK(rule_type IN ('SURCHARGE', 'DISCOUNT')),
    CHECK(scope IN ('GLOBAL', 'CATEGORY', 'TAG', 'PRODUCT', 'ZONE')),
    CHECK(adjustment_type IN ('PERCENTAGE', 'FIXED_AMOUNT')),
    CHECK(time_mode IN ('ALWAYS', 'SCHEDULE', 'ONETIME')),
    CHECK(is_stackable IN (0, 1)),
    CHECK(json_valid(schedule_config_json) OR schedule_config_json IS NULL)
);

CREATE INDEX IF NOT EXISTS idx_pa_active ON price_adjustment_rules(is_active, scope, time_mode);

-- =============================================================================
-- 10. 订单 (只读模式: 不允许删除/修改，只允许新增)
-- =============================================================================
-- 哈希链保护:
-- - order.prev_hash = 上一个订单的 curr_hash (或 genesis_hash)
-- - order.curr_hash = SHA256(order数据 + event[last].curr_hash)
-- - event[0].prev_hash = order.receipt_number
-- - event[n].prev_hash = event[n-1].curr_hash
-- - event[n].curr_hash = SHA256(event[n]数据 + prev_hash)

CREATE TABLE IF NOT EXISTS orders (
    order_id INTEGER PRIMARY KEY AUTOINCREMENT,
    receipt_number TEXT NOT NULL UNIQUE,
    table_id INTEGER,
    table_name TEXT,
    status TEXT NOT NULL,
    start_time INTEGER,
    end_time INTEGER,
    guest_count INTEGER,
    subtotal INTEGER,
    total INTEGER,
    discount_type TEXT,
    discount_value REAL,
    discount_amount INTEGER,
    surcharge_amount INTEGER,
    surcharge_total INTEGER,
    zone_name TEXT,
    prev_hash TEXT NOT NULL,
    curr_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_orders_end_time ON orders(end_time);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_table_id ON orders(table_id);
CREATE INDEX IF NOT EXISTS idx_orders_status_endtime ON orders(status, end_time DESC);
CREATE INDEX IF NOT EXISTS idx_orders_prev_hash ON orders(prev_hash);
CREATE INDEX IF NOT EXISTS idx_orders_curr_hash ON orders(curr_hash);

-- 订单事件表
CREATE TABLE IF NOT EXISTS orders_events (
    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    title TEXT,
    summary TEXT,
    note TEXT,
    color TEXT,
    data_json TEXT,
    prev_hash TEXT NOT NULL,
    curr_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY(order_id) REFERENCES orders(order_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_events_order ON orders_events(order_id);
CREATE INDEX IF NOT EXISTS idx_events_time ON orders_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_curr_hash ON orders_events(curr_hash);

-- 订单明细表
CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    order_id INTEGER NOT NULL,
    product_id TEXT,
    specification_id INTEGER,
    receipt_name TEXT,
    name TEXT,
    price INTEGER,
    quantity INTEGER,
    discount_amount INTEGER DEFAULT 0,
    surcharge_amount INTEGER DEFAULT 0,
    original_price INTEGER,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY(order_id) REFERENCES orders(order_id) ON DELETE RESTRICT,
    FOREIGN KEY(product_id) REFERENCES products(id),
    FOREIGN KEY(specification_id) REFERENCES product_specifications(id)
);

CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_product ON order_items(product_id);

-- 订单明细选项表
CREATE TABLE IF NOT EXISTS order_item_options (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_item_id INTEGER NOT NULL,
    attribute_id INTEGER NOT NULL,
    attribute_name TEXT NOT NULL,
    option_id TEXT NOT NULL,
    option_name TEXT NOT NULL,
    price_modifier INTEGER DEFAULT 0,
    receipt_name TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY(order_item_id) REFERENCES order_items(id) ON DELETE RESTRICT
);

-- 支付记录表
CREATE TABLE IF NOT EXISTS payments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    order_id INTEGER NOT NULL,
    method TEXT,
    amount INTEGER,
    timestamp INTEGER,
    note TEXT,
    card_brand TEXT,
    last4 TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    deleted_at INTEGER,
    deleted_by TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY(order_id) REFERENCES orders(order_id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_payments_order ON payments(order_id);
CREATE INDEX IF NOT EXISTS idx_payments_method ON payments(method);

-- =============================================================================
-- 11. 系统状态 (哈希链状态缓存)
-- =============================================================================
-- 用于本地验证和离线模式
-- genesis_hash 和 last_order_hash 由服务端首次同步时分配

CREATE TABLE IF NOT EXISTS system_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    genesis_hash TEXT,
    last_order_id INTEGER NOT NULL DEFAULT 0,
    last_order_hash TEXT,
    synced_up_to_id INTEGER NOT NULL DEFAULT 0,
    synced_up_to_hash TEXT,
    last_sync_time INTEGER,
    order_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

INSERT OR IGNORE INTO system_state (id, last_order_id, last_order_hash, synced_up_to_id, synced_up_to_hash, order_count) VALUES
(1, 0, NULL, 0, NULL, 0);

-- =============================================================================
-- 12. 审计日志
-- =============================================================================
-- 用于税务审计追溯，所有消息使用西班牙语

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT UNIQUE NOT NULL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    category TEXT NOT NULL,
    event_type TEXT NOT NULL,
    user_id TEXT,
    username TEXT,
    entity_type TEXT,
    entity_id TEXT,
    entity_name TEXT,
    action TEXT NOT NULL,
    description TEXT,
    severity TEXT NOT NULL DEFAULT 'INFO',
    metadata_json TEXT,
    source TEXT,
    source_device TEXT,
    source_ip TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    CHECK(category IN ('SYSTEM', 'OPERATION', 'SECURITY', 'DATA', 'PAYMENT', 'PRINT')),
    CHECK(severity IN ('DEBUG', 'INFO', 'WARNING', 'ERROR', 'CRITICAL'))
);

CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_category ON audit_logs(category);
CREATE INDEX IF NOT EXISTS idx_audit_event_type ON audit_logs(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_logs(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_audit_severity ON audit_logs(severity);

-- =============================================================================
-- 13. 触发器
-- =============================================================================
-- 13.1 保护核心订单状态
CREATE TRIGGER IF NOT EXISTS trigger_orders_update_protect
BEFORE UPDATE ON orders
FOR EACH ROW
BEGIN
    -- 状态保护
    SELECT RAISE(ABORT, 'El estado del pedido no se puede modificar directamente, use eventos del pedido')
    WHERE OLD.status != NEW.status;

    -- 总计保护
    SELECT RAISE(ABORT, 'El total del pedido no se puede modificar directamente')
    WHERE OLD.total != NEW.total;
END;

-- 13.4 业务规则保护（删除限制）
CREATE TRIGGER IF NOT EXISTS trigger_product_delete_protect
BEFORE DELETE ON products
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'No se puede eliminar el producto con pedidos activos')
    WHERE EXISTS (SELECT 1 FROM order_items WHERE product_id = OLD.id AND is_deleted = 0);
END;

CREATE TRIGGER IF NOT EXISTS trigger_category_delete_protect
BEFORE DELETE ON categories
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'No se puede eliminar la categoria con productos activos')
    WHERE EXISTS (SELECT 1 FROM products WHERE category_id = OLD.id AND is_deleted = 0);
END;

CREATE TRIGGER IF NOT EXISTS trigger_role_delete_protect
BEFORE DELETE ON roles
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'No se puede eliminar el rol con usuarios activos')
    WHERE EXISTS (SELECT 1 FROM users WHERE role_id = OLD.id AND is_deleted = 0);
END;

CREATE TRIGGER IF NOT EXISTS trigger_role_system_protect
BEFORE UPDATE ON roles
FOR EACH ROW
BEGIN
    SELECT RAISE(ABORT, 'No se puede modificar el rol integrado del sistema')
    WHERE OLD.is_system = 1 AND (OLD.name != NEW.name OR OLD.is_system != NEW.is_system);
END;

-- =============================================================================
-- 14. 示例数据
-- =============================================================================

-- 角色
INSERT OR IGNORE INTO roles (uuid, name, display_name, description, is_system) VALUES
('role_admin', 'admin', 'Administrador', 'Administrador del sistema con todos los permisos', 1);
-- 用户 (密码: admin123)
INSERT OR IGNORE INTO users (uuid, username, password_hash, display_name, role_id) VALUES
('usr_admin', 'admin', '$2b$12$zmNK7Eh1QvSXIaOlbpPPfOS1JYgU9Tpuc.xO383h1AwdRSkpWbWdm', 'Administrador', 1);
-- 权限
INSERT OR IGNORE INTO role_permissions (role_id, permission) VALUES
(1, 'all');

-- 打印机
INSERT OR IGNORE INTO kitchen_printers (name, printer_name, description) VALUES
('Cocina Principal', 'Kitchen_Printer_1', 'Impresora de area de alimentos principales'),
('Bebidas', 'Kitchen_Printer_2', 'Impresora de area de bebidas');

-- 分类
INSERT OR IGNORE INTO categories (uuid, name, sort_order, kitchen_printer_id, is_kitchen_print_enabled, is_label_print_enabled) VALUES
('cat_popular', 'Populares', 1, 1, 1, 1),
('cat_drinks', 'Bebidas', 2, 2, 1, 1),
('cat_snacks', 'Aperitivos', 3, 1, 1, 1),
('cat_main', 'Platos Principales', 4, 1, 1, 1);

-- 区域
INSERT OR IGNORE INTO zones (uuid, name, description) VALUES
('zone_main', 'Salon Principal', 'Area principal de comedor'),
('zone_terrace', 'Terraza', 'Area de asientos exterior');

-- 桌台
INSERT OR IGNORE INTO tables (uuid, name, zone_id, capacity) VALUES
('tbl_1', 'Mesa 1', 1, 4),
('tbl_2', 'Mesa 2', 1, 4),
('tbl_3', 'Mesa 3', 2, 6),
('tbl_4', 'Mesa 4', 2, 2);


-- =============================================================================
-- 完成
-- =============================================================================
-- 总表数: 20 张表
-- 触发器数: 14 个
