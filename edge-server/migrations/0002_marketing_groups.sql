-- ── Marketing Groups (营销组 = 会员等级) ─────────────────────
CREATE TABLE marketing_group (
    id           INTEGER PRIMARY KEY,
    name         TEXT    NOT NULL UNIQUE,
    display_name TEXT    NOT NULL,
    description  TEXT,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    points_earn_rate  REAL,
    points_per_unit   REAL,
    is_active    INTEGER NOT NULL DEFAULT 1,
    created_at   INTEGER NOT NULL DEFAULT 0,
    updated_at   INTEGER NOT NULL DEFAULT 0
);

-- ── Members (会员) ──────────────────────────────────────────
CREATE TABLE member (
    id                 INTEGER PRIMARY KEY,
    name               TEXT    NOT NULL,
    phone              TEXT,
    card_number        TEXT,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    birthday           TEXT,
    points_balance     INTEGER NOT NULL DEFAULT 0,
    notes              TEXT,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_member_phone ON member(phone);
CREATE INDEX idx_member_card_number ON member(card_number);
CREATE INDEX idx_member_marketing_group ON member(marketing_group_id);

-- ── MG Discount Rules (MG 折扣规则) ────────────────────────
CREATE TABLE mg_discount_rule (
    id                 INTEGER PRIMARY KEY,
    marketing_group_id INTEGER NOT NULL REFERENCES marketing_group(id),
    name               TEXT    NOT NULL,
    display_name       TEXT    NOT NULL,
    receipt_name       TEXT    NOT NULL,
    product_scope      TEXT    NOT NULL,
    target_id          INTEGER,
    adjustment_type    TEXT    NOT NULL,
    adjustment_value   REAL    NOT NULL,
    is_active          INTEGER NOT NULL DEFAULT 1,
    created_at         INTEGER NOT NULL DEFAULT 0,
    updated_at         INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_mg_discount_rule_group ON mg_discount_rule(marketing_group_id);

-- ── Stamp Activities (集章活动) ─────────────────────────────
CREATE TABLE stamp_activity (
    id                    INTEGER PRIMARY KEY,
    marketing_group_id    INTEGER NOT NULL REFERENCES marketing_group(id),
    name                  TEXT    NOT NULL,
    display_name          TEXT    NOT NULL,
    stamps_required       INTEGER NOT NULL,
    reward_quantity       INTEGER NOT NULL DEFAULT 1,
    reward_strategy       TEXT    NOT NULL DEFAULT 'ECONOMIZADOR',
    designated_product_id INTEGER,
    is_cyclic             INTEGER NOT NULL DEFAULT 1,
    is_active             INTEGER NOT NULL DEFAULT 1,
    created_at            INTEGER NOT NULL DEFAULT 0,
    updated_at            INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_stamp_activity_group ON stamp_activity(marketing_group_id);

-- ── Stamp Targets (集章条件目标) ────────────────────────────
CREATE TABLE stamp_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_target_activity ON stamp_target(stamp_activity_id);

-- ── Stamp Reward Targets (集章奖励目标) ─────────────────────
CREATE TABLE stamp_reward_target (
    id                INTEGER PRIMARY KEY,
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id) ON DELETE CASCADE,
    target_type       TEXT    NOT NULL,
    target_id         INTEGER NOT NULL
);
CREATE INDEX idx_stamp_reward_target_activity ON stamp_reward_target(stamp_activity_id);

-- ── Member Stamp Progress (会员集章进度) ────────────────────
CREATE TABLE member_stamp_progress (
    id                INTEGER PRIMARY KEY,
    member_id         INTEGER NOT NULL REFERENCES member(id),
    stamp_activity_id INTEGER NOT NULL REFERENCES stamp_activity(id),
    current_stamps    INTEGER NOT NULL DEFAULT 0,
    completed_cycles  INTEGER NOT NULL DEFAULT 0,
    last_stamp_at     INTEGER,
    updated_at        INTEGER NOT NULL DEFAULT 0,
    UNIQUE(member_id, stamp_activity_id)
);
