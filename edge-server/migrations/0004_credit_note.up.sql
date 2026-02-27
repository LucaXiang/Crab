-- =============================================================================
-- credit_note + credit_note_item: 退款凭证
-- =============================================================================

CREATE TABLE credit_note (
    id                    INTEGER PRIMARY KEY,
    credit_note_number    TEXT    NOT NULL,
    original_order_pk     INTEGER NOT NULL REFERENCES archived_order(id),
    original_receipt      TEXT    NOT NULL,

    -- 金额（正数，表示退了多少）
    subtotal_credit       REAL    NOT NULL,
    tax_credit            REAL    NOT NULL,
    total_credit          REAL    NOT NULL,

    -- 退款方式
    refund_method         TEXT    NOT NULL,

    -- 审计
    reason                TEXT    NOT NULL,
    note                  TEXT,
    operator_id           INTEGER NOT NULL,
    operator_name         TEXT    NOT NULL,
    authorizer_id         INTEGER,
    authorizer_name       TEXT,

    -- 归属
    shift_id              INTEGER REFERENCES shift(id),
    cloud_synced          INTEGER NOT NULL DEFAULT 0,
    created_at            INTEGER NOT NULL
);

CREATE UNIQUE INDEX idx_cn_number ON credit_note(credit_note_number);
CREATE INDEX idx_cn_original ON credit_note(original_order_pk);
CREATE INDEX idx_cn_created ON credit_note(created_at);
CREATE INDEX idx_cn_cloud_synced ON credit_note(cloud_synced);
CREATE INDEX idx_cn_shift ON credit_note(shift_id);

CREATE TABLE credit_note_item (
    id                    INTEGER PRIMARY KEY,
    credit_note_id        INTEGER NOT NULL REFERENCES credit_note(id),
    original_instance_id  TEXT    NOT NULL,
    item_name             TEXT    NOT NULL,
    quantity              INTEGER NOT NULL,
    unit_price            REAL    NOT NULL,
    line_credit           REAL    NOT NULL,
    tax_rate              INTEGER NOT NULL,
    tax_credit            REAL    NOT NULL
);

CREATE INDEX idx_cni_credit_note ON credit_note_item(credit_note_id);
