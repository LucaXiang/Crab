UPDATE role:admin CONTENT {
	is_active: true,
	is_system: true,
	permissions: [
		"*",
	],
	role_name: 'admin',
};

UPDATE employee:admin CONTENT {
    username: 'admin',
    hash_pass: '$argon2id$v=19$m=19456,t=2,p=1$ufh/dTiLBQM6xDgFXG/p2Q$n4Z6pHZkJqhBcXnhOA/QHqu4a5tI7fPdWfYm1fkxh40',
    employee_name: 'admin',
    role: role:admin,
    is_system: true,
    is_active: true,
};


