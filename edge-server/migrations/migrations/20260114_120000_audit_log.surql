-- Audit Log Schema for Edge Server
-- Tracks all important system events for compliance and debugging

-- Define audit_log table
DEFINE TABLE OVERWRITE audit_log TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR create FULL
        FOR update, delete NONE;

-- Core fields
DEFINE FIELD OVERWRITE action ON audit_log TYPE string
    ASSERT $value IN [
        'login', 'logout', 'login_failed', 'token_refresh',
        'read', 'create', 'update', 'delete',
        'config_change', 'user_create', 'user_update', 'user_delete',
        'role_change', 'permission_change',
        'server_start', 'server_stop', 'backup_create', 'backup_restore',
        'unauthorized_access', 'invalid_token'
    ]
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE resource ON audit_log TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE success ON audit_log TYPE bool DEFAULT true
    PERMISSIONS FULL;

-- User context (optional - may be null for anonymous actions)
DEFINE FIELD OVERWRITE user_id ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE username ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- Request context
DEFINE FIELD OVERWRITE ip_address ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE user_agent ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- Additional details
DEFINE FIELD OVERWRITE details ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE error ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- Timestamps
DEFINE FIELD OVERWRITE created_at ON audit_log TYPE datetime VALUE time::now() READONLY
    PERMISSIONS FULL;

-- Indexes for common queries
DEFINE INDEX OVERWRITE audit_action_idx ON audit_log FIELDS action;
DEFINE INDEX OVERWRITE audit_user_idx ON audit_log FIELDS user_id;
DEFINE INDEX OVERWRITE audit_resource_idx ON audit_log FIELDS resource;
DEFINE INDEX OVERWRITE audit_created_idx ON audit_log FIELDS created_at;
DEFINE INDEX OVERWRITE audit_success_idx ON audit_log FIELDS success;
