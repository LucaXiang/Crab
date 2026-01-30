-- Daily Report Schema
-- 日结报告表

DEFINE TABLE OVERWRITE daily_report TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 营业日期 (YYYY-MM-DD)
DEFINE FIELD OVERWRITE business_date ON daily_report TYPE string
    ASSERT string::len($value) = 10
    PERMISSIONS FULL;

-- === 订单统计 ===

-- 总订单数
DEFINE FIELD OVERWRITE total_orders ON daily_report TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 完成订单数
DEFINE FIELD OVERWRITE completed_orders ON daily_report TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 作废订单数
DEFINE FIELD OVERWRITE void_orders ON daily_report TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- === 金额统计 ===

-- 总营业额 (completed orders)
DEFINE FIELD OVERWRITE total_sales ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 已支付金额
DEFINE FIELD OVERWRITE total_paid ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 未支付金额
DEFINE FIELD OVERWRITE total_unpaid ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 作废订单金额
DEFINE FIELD OVERWRITE void_amount ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 税额总计
DEFINE FIELD OVERWRITE total_tax ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 折扣总计
DEFINE FIELD OVERWRITE total_discount ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 附加费总计
DEFINE FIELD OVERWRITE total_surcharge ON daily_report TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- === 税率分类 (西班牙 IVA) ===
DEFINE FIELD OVERWRITE tax_breakdowns ON daily_report TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tax_breakdowns.*.tax_rate ON daily_report TYPE int PERMISSIONS FULL;
DEFINE FIELD OVERWRITE tax_breakdowns.*.net_amount ON daily_report TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE tax_breakdowns.*.tax_amount ON daily_report TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE tax_breakdowns.*.gross_amount ON daily_report TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE tax_breakdowns.*.order_count ON daily_report TYPE int DEFAULT 0 PERMISSIONS FULL;

-- === 支付方式分类 ===
DEFINE FIELD OVERWRITE payment_breakdowns ON daily_report TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE payment_breakdowns.*.method ON daily_report TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE payment_breakdowns.*.amount ON daily_report TYPE float DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE payment_breakdowns.*.count ON daily_report TYPE int DEFAULT 0 PERMISSIONS FULL;

-- === 生成信息 ===

-- 生成时间 (Unix timestamp millis)
DEFINE FIELD OVERWRITE generated_at ON daily_report TYPE option<int>
    PERMISSIONS FULL;

-- 生成人 ID
DEFINE FIELD OVERWRITE generated_by_id ON daily_report TYPE option<string>
    PERMISSIONS FULL;

-- 生成人姓名
DEFINE FIELD OVERWRITE generated_by_name ON daily_report TYPE option<string>
    PERMISSIONS FULL;

-- 备注
DEFINE FIELD OVERWRITE note ON daily_report TYPE option<string>
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE daily_report_date_unique ON daily_report FIELDS business_date UNIQUE;
