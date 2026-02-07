-- Product Spec Index Table
-- 不再需要：external_id 已移至 product 表级别，通过 UNIQUE INDEX 直接约束
-- 保留此文件以确保 REMOVE TABLE 清理
REMOVE TABLE IF EXISTS product_spec;
