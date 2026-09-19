do
$$
    begin
        create type token_type as enum ('personal', 'organization', 'ci', 'deploy', 'runner', 'instance');
    exception
        when duplicate_object then null;
    end
$$;

do
$$
    begin
        create type token_permissions as enum (
            'contents:read',
            'contents:write',
            'repo:read',
            'repo:write',
            'repo:create',
            'repo:delete',
            'repo:admin',
            'releases:read',
            'releases:write',
            'releases:assets:write',
            'issues:read',
            'issues:write',
            'issues:comment',
            'issues:labels',
            'issues:milestones',
            'org:read',
            'org:write',
            'org:create',
            'org:delete',
            'org:members:read',
            'org:members:write',
            'user:read',
            'user:write',
            'user:email:read',
            'ssh_keys:read',
            'sessions:read',
            'stars:read',
            'stars:write',
            'events:read',
            'audit_log:read',
            'org:audit_log:read',
            'admin:users:read',
            'admin:users:write',
            'admin:stats:read',
            'admin:health:read',
            'admin:audit_log:read',
            'admin:settings:read',
            'admin:settings:write'
        );
    exception
        when duplicate_object then null;
    end
$$;

do
$$
    begin
        create type token_scope as enum ('all', 'public', 'selected');
    exception
        when duplicate_object then null;
    end
$$;

create table if not exists tokens
(
    id           uuid                not null
        constraint tokens_pk
            primary key,
    name         text                not null,
    token        bytea               not null
        constraint tokens_token_uq
            unique,
    owner_user   uuid       default null
        constraint tokens_users_owner_id_fk
            references users
            on delete cascade,
    owner_org    uuid       default null
        constraint tokens_organizations_id_fk
            references organizations
            on delete cascade,
    owner_repo   uuid       default null
        constraint tokens_repositories_id_fk
            references repositories
            on delete cascade,
    creator      uuid       default null
        constraint tokens_users_creator_id_fk
            references users
            on delete set null,
    token_type   token_type          not null,
    scope        token_scope         not null,
    permissions  token_permissions[] not null,
    last_used_at timestamptz default null,
    expires_at   timestamptz default null,
    revoked_at   timestamptz default null,

    unique (id, token_type)
);

alter table tokens drop constraint if exists tokens_owner_exclusive;
alter table tokens add constraint tokens_owner_exclusive
    check (
        case token_type
            when 'personal' then owner_user is not null and owner_org is null and owner_repo is null
            when 'organization' then owner_org is not null and owner_user is null and owner_repo is null
            when 'ci' then owner_repo is not null and owner_user is null and owner_org is null
            when 'deploy' then owner_repo is not null and owner_user is null and owner_org is null
            when 'runner' then num_nonnulls(owner_user, owner_org, owner_repo) <= 1
            when 'instance' then num_nonnulls(owner_user, owner_org, owner_repo) = 0
            else
                false
            end
        );

alter table tokens drop constraint if exists tokens_scope_valid;
alter table tokens add constraint tokens_scope_valid
    check (
        case token_type
            when 'personal' then true
            when 'organization' then scope in ('all', 'selected')
            else scope = 'all'
            end
    );

alter table tokens drop constraint if exists tokens_permissions_not_empty;
alter table tokens add constraint tokens_permissions_not_empty
    check (cardinality(permissions) > 0);

alter table tokens drop constraint if exists tokens_token_length;
alter table tokens add constraint tokens_token_length
    check (octet_length(token) = 32);

create unique index if not exists tokens_owner_user_name_uq on tokens (owner_user, name) where owner_user is not null and revoked_at is null;
create unique index if not exists tokens_owner_org_name_uq on tokens (owner_org, name) where owner_org is not null and revoked_at is null;
create unique index if not exists tokens_owner_repo_name_uq on tokens (owner_repo, name) where owner_repo is not null and revoked_at is null;
create unique index if not exists tokens_ownerless_name_uq on tokens (name) where owner_user is null and owner_org is null and owner_repo is null and revoked_at is null;

create index if not exists tokens_owner_user_idx on tokens (owner_user) where owner_user is not null;
create index if not exists tokens_owner_org_idx on tokens (owner_org) where owner_org is not null;
create index if not exists tokens_owner_repo_idx on tokens (owner_repo) where owner_repo is not null;
create index if not exists tokens_creator_idx on tokens (creator) where creator is not null;
create index if not exists tokens_permissions_idx on tokens using gin (permissions);

create table if not exists token_scopes
(
    token_id   uuid       not null,
    token_type token_type not null,
    scope_org  uuid default null
        references organizations
            on delete cascade,
    scope_repo uuid default null
        references repositories
            on delete cascade,

    foreign key (token_id, token_type) references tokens (id, token_type) on delete cascade
);

alter table token_scopes drop constraint if exists token_scopes_target_exclusive;
alter table token_scopes add constraint token_scopes_target_exclusive
    check (num_nonnulls(scope_org, scope_repo) = 1);

alter table token_scopes drop constraint if exists token_scopes_kind_allowed;
alter table token_scopes add constraint token_scopes_kind_allowed
    check (
        case token_type
            when 'personal' then true
            when 'organization' then scope_repo is not null
            else
                false
            end
    );

create unique index if not exists token_scopes_target_uq on token_scopes (token_id, coalesce(scope_org, scope_repo));
create index if not exists token_scopes_org_idx on token_scopes (scope_org) where scope_org is not null;
create index if not exists token_scopes_repo_idx on token_scopes (scope_repo) where scope_repo is not null;
