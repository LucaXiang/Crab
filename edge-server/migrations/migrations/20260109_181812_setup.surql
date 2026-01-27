CREATE role:admin CONTENT {
	name: 'admin',
	display_name: '管理员',
	description: '系统管理员，拥有全部权限',
	permissions: [
		"*",
	],
	is_system: true,
	is_active: true,
};

CREATE employee:admin CONTENT {
    username: 'admin',
    hash_pass: '$argon2id$v=19$m=19456,t=2,p=1$4K7SyBwr5d3uF4hroPQf2w$hPqq7x5rSE1d9TTf+hK3eipuaeeElC7GMHuSJIozDws',
    display_name: '管理员',
    role: role:admin,
    is_system: true,
    is_active: true,
};
