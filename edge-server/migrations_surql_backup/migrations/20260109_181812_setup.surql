CREATE role:admin CONTENT {
	name: 'admin',
	display_name: 'admin',
	description: 'administrator',
	permissions: [
		"*",
	],
	is_system: true,
	is_active: true,
};

CREATE employee:admin CONTENT {
    username: 'admin',
    hash_pass: '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws',
    display_name: 'admin',
    role: role:admin,
    is_system: true,
    is_active: true,
    created_at: time::unix(time::now()) * 1000,
};

-- 单例表初始化
CREATE store_info:main CONTENT {
    name: '',
    address: '',
    nif: '',
    business_day_cutoff: '00:00',
    created_at: time::unix(time::now()) * 1000,
    updated_at: time::unix(time::now()) * 1000,
};

CREATE system_state:main CONTENT {
    order_count: 0,
    created_at: time::unix(time::now()) * 1000,
    updated_at: time::unix(time::now()) * 1000,
};
