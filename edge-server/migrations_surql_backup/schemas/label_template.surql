-- Label Template Schema
-- 标签打印模板表

DEFINE TABLE OVERWRITE label_template TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 模板名称
DEFINE FIELD OVERWRITE name ON label_template TYPE string
    PERMISSIONS FULL;

-- 模板描述
DEFINE FIELD OVERWRITE description ON label_template TYPE option<string>
    PERMISSIONS FULL;

-- 标签宽度 (mm)
DEFINE FIELD OVERWRITE width ON label_template TYPE float
    PERMISSIONS FULL;

-- 标签高度 (mm)
DEFINE FIELD OVERWRITE height ON label_template TYPE float
    PERMISSIONS FULL;

-- 内边距 (mm)
DEFINE FIELD OVERWRITE padding ON label_template TYPE float
    DEFAULT 2.0
    PERMISSIONS FULL;

-- 模板字段列表 (嵌入式数组)
DEFINE FIELD OVERWRITE fields ON label_template TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE fields.*.id ON label_template TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.name ON label_template TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.type ON label_template TYPE string
    PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.x ON label_template TYPE float PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.y ON label_template TYPE float PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.width ON label_template TYPE float PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.height ON label_template TYPE float PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.font_size ON label_template TYPE int DEFAULT 10 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.font_weight ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.font_family ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.color ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.rotate ON label_template TYPE option<int> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.alignment ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.data_source ON label_template TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.format ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.visible ON label_template TYPE bool DEFAULT true PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.label ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.template ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.data_key ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.source_type ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.maintain_aspect_ratio ON label_template TYPE option<bool> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.style ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.align ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.vertical_align ON label_template TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE fields.*.line_style ON label_template TYPE option<string> PERMISSIONS FULL;

-- 是否默认模板
DEFINE FIELD OVERWRITE is_default ON label_template TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 是否启用
DEFINE FIELD OVERWRITE is_active ON label_template TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- UI 兼容字段
DEFINE FIELD OVERWRITE width_mm ON label_template TYPE option<float> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE height_mm ON label_template TYPE option<float> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE padding_mm_x ON label_template TYPE option<float> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE padding_mm_y ON label_template TYPE option<float> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE render_dpi ON label_template TYPE option<int> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE test_data ON label_template TYPE option<string> PERMISSIONS FULL;

-- 时间戳
DEFINE FIELD OVERWRITE created_at ON label_template TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON label_template TYPE option<int>
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE label_template_name_idx ON label_template FIELDS name;
DEFINE INDEX OVERWRITE label_template_active_idx ON label_template FIELDS is_active;
