CREATE role:admin CONTENT {
	is_active: true,
	is_system: true,
	permissions: [
		"*",
	],
	role_name: 'admin',
};

CREATE employee:admin CONTENT {
    username: 'admin',
    hash_pass: '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws',
    employee_name: 'admin',
    role: role:admin,
    is_system: true,
    is_active: true,
};


