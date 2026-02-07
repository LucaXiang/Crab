-- Payment 独立表：支付记录（从订单事件溯源投影）
-- 归档时写入，方便统计和对账

DEFINE TABLE OVERWRITE payment TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update WHERE true
        FOR delete NONE;

-- 核心字段
DEFINE FIELD OVERWRITE payment_id    ON payment TYPE string;
DEFINE FIELD OVERWRITE order_id      ON payment TYPE string;
DEFINE FIELD OVERWRITE method        ON payment TYPE string;        -- CASH / CARD
DEFINE FIELD OVERWRITE amount        ON payment TYPE decimal;
DEFINE FIELD OVERWRITE tendered      ON payment TYPE option<decimal>; -- 现金实收
DEFINE FIELD OVERWRITE change_amount ON payment TYPE option<decimal>; -- 找零
DEFINE FIELD OVERWRITE note          ON payment TYPE option<string>;

-- 分单上下文
DEFINE FIELD OVERWRITE split_type    ON payment TYPE option<string>; -- ItemSplit / AmountSplit / AaSplit / null
DEFINE FIELD OVERWRITE aa_shares     ON payment TYPE option<int>;
DEFINE FIELD OVERWRITE split_items   ON payment TYPE option<string>; -- JSON string [{name, quantity, unit_price}]

-- 操作人
DEFINE FIELD OVERWRITE operator_id   ON payment TYPE option<string>;
DEFINE FIELD OVERWRITE operator_name ON payment TYPE option<string>;

-- 取消
DEFINE FIELD OVERWRITE cancelled     ON payment TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE cancel_reason ON payment TYPE option<string>;

-- 时间
DEFINE FIELD OVERWRITE timestamp     ON payment TYPE int;   -- 支付时间 (unix ms)
DEFINE FIELD OVERWRITE created_at    ON payment TYPE int;    -- 记录创建时间

-- 索引
DEFINE INDEX OVERWRITE idx_payment_id  ON payment FIELDS payment_id UNIQUE;
DEFINE INDEX OVERWRITE idx_order_id    ON payment FIELDS order_id;
DEFINE INDEX OVERWRITE idx_timestamp   ON payment FIELDS timestamp;
DEFINE INDEX OVERWRITE idx_operator    ON payment FIELDS operator_id;
