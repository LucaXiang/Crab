-- ============================================================
-- Missing FK indexes + safety constraints
-- ============================================================

-- FK indexes for JOIN performance
CREATE INDEX IF NOT EXISTS idx_employee_role ON employee(role_id);
CREATE INDEX IF NOT EXISTS idx_attr_binding_attribute ON attribute_binding(attribute_id);
CREATE INDEX IF NOT EXISTS idx_price_rule_creator ON price_rule(created_by);

-- Prevent duplicate open shifts per operator (race condition safety)
CREATE UNIQUE INDEX IF NOT EXISTS idx_shift_operator_open
    ON shift(operator_id) WHERE status = 'OPEN';
