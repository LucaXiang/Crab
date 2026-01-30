-- Payment 独立表：支付记录（从订单事件溯源投影）
-- 归档时写入，方便统计和对账

DEFINE TABLE IF NOT EXISTS payment TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update WHERE true
        FOR delete NONE;

-- 核心字段
DEFINE FIELD IF NOT EXISTS payment_id    ON payment TYPE string;
DEFINE FIELD IF NOT EXISTS order_id      ON payment TYPE string;
DEFINE FIELD IF NOT EXISTS method        ON payment TYPE string;        -- CASH / CARD
DEFINE FIELD IF NOT EXISTS amount        ON payment TYPE float;
DEFINE FIELD IF NOT EXISTS tendered      ON payment TYPE option<float>; -- 现金实收
DEFINE FIELD IF NOT EXISTS change_amount ON payment TYPE option<float>; -- 找零
DEFINE FIELD IF NOT EXISTS note          ON payment TYPE option<string>;

-- 分单上下文
DEFINE FIELD IF NOT EXISTS split_type    ON payment TYPE option<string>; -- ItemSplit / AmountSplit / AaSplit / null
DEFINE FIELD IF NOT EXISTS aa_shares     ON payment TYPE option<int>;
DEFINE FIELD IF NOT EXISTS split_items   ON payment TYPE option<string>; -- JSON string [{name, quantity, unit_price}]

-- 操作人
DEFINE FIELD IF NOT EXISTS operator_id   ON payment TYPE option<string>;
DEFINE FIELD IF NOT EXISTS operator_name ON payment TYPE option<string>;

-- 取消
DEFINE FIELD IF NOT EXISTS cancelled     ON payment TYPE bool DEFAULT false;
DEFINE FIELD IF NOT EXISTS cancel_reason ON payment TYPE option<string>;

-- 时间
DEFINE FIELD IF NOT EXISTS timestamp     ON payment TYPE int;   -- 支付时间 (unix ms)
DEFINE FIELD IF NOT EXISTS created_at    ON payment TYPE int;    -- 记录创建时间

-- 索引
DEFINE INDEX IF NOT EXISTS idx_payment_id  ON payment FIELDS payment_id UNIQUE;
DEFINE INDEX IF NOT EXISTS idx_order_id    ON payment FIELDS order_id;
DEFINE INDEX IF NOT EXISTS idx_method      ON payment FIELDS method;
DEFINE INDEX IF NOT EXISTS idx_split_type  ON payment FIELDS split_type;
DEFINE INDEX IF NOT EXISTS idx_timestamp   ON payment FIELDS timestamp;
DEFINE INDEX IF NOT EXISTS idx_operator    ON payment FIELDS operator_id;
DEFINE INDEX IF NOT EXISTS idx_cancelled   ON payment FIELDS cancelled;
