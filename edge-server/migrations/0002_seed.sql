-- Seed data: admin role, admin user, store_info, system_state

INSERT INTO role (id, name, display_name, description, permissions, is_system, is_active)
VALUES (1, 'admin', 'admin', 'administrator', '["*"]', 1, 1);

-- password: 'admin' (Argon2id hash)
INSERT INTO employee (id, username, hash_pass, display_name, role_id, is_system, is_active, created_at)
VALUES (1, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws', 'admin', 1, 1, 1, 0);

INSERT INTO store_info (id, name, address, nif, business_day_cutoff, created_at, updated_at)
VALUES (1, '', '', '', '00:00', 0, 0);

INSERT INTO system_state (id, order_count, created_at, updated_at)
VALUES (1, 0, 0, 0);
