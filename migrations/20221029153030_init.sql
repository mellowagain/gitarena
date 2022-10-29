-- STYLE: SQL keywords are always lowercase. Names follow snake_case. Every statement needs to end in an semicolon.

-- Users

create table if not exists users
(
    id         serial                                                               not null
    constraint users_pk
    primary key,
    username   varchar(32)                                                          not null,
    password   char(96)                                                             not null,
    disabled   boolean                  default false                               not null,
    admin      boolean                  default false                               not null,
    created_at timestamp with time zone default current_timestamp                   not null
);

create unique index if not exists users_username_uindex
    on users (username);

-- Emails

create table if not exists emails
(
    id              serial                                              not null
        constraint emails_pk
            primary key,
    owner           integer                                             not null
        constraint emails_users_id_fk
            references users
            on delete cascade,
    email           varchar(256)                                        not null,
    "primary"       boolean default false                               not null,
    commit          boolean default false                               not null,
    notification    boolean default false                               not null,
    public          boolean default false                               not null,
    created_at      timestamp with time zone default current_timestamp  not null,
    verified_at     timestamp with time zone
);

create unique index if not exists emails_email_uindex
    on emails (email);

create index if not exists emails_owner_index
    on emails (owner);

-- User Verifications

create table if not exists user_verifications
(
    id      serial                   not null
    constraint user_verifications_pk
    primary key,
    user_id integer                  not null,
    hash    char(32)                 not null,
    expires timestamp with time zone not null
                          );

create unique index if not exists user_verifications_hash_uindex
    on user_verifications (hash);

create unique index if not exists user_verifications_user_id_uindex
    on user_verifications (user_id);

-- Repositories

-- https://stackoverflow.com/a/48382296/11494565
do $$ begin
    create type repo_visibility as enum ('public', 'internal', 'private');
exception
      when duplicate_object then null;
end $$;

create table if not exists repositories
(
    id             serial                                               not null
    constraint repositories_pk
    primary key,
    owner          integer                                              not null
    constraint repositories_users_id_fk
    references users (id)
    on delete cascade,
    name           varchar(32)                                          not null,
    description    varchar(256)                                         not null,
    visibility     repo_visibility default 'public'::repo_visibility    not null,
    default_branch varchar(256) default 'main'::character varying       not null,
    license        varchar(256) default NULL::character varying,
    forked_from    integer,
    mirrored_from  varchar(256) default NULL::character varying,
    archived       boolean default false                                not null,
    disabled       boolean default false                                not null
    );

-- Privileges

-- https://stackoverflow.com/a/48382296/11494565
do $$ begin
    create type access_level as enum ('viewer', 'supporter', 'coder', 'manager', 'admin');
exception
    when duplicate_object then null;
end $$;

create table if not exists privileges
(
    id           serial
    constraint privileges_pk
    primary key,
    user_id      integer                                     not null
    constraint privileges_users_id_fk
    references users
    on delete cascade,
    repo_id      integer                                     not null
    constraint privileges_repositories_id_fk
    references repositories
    on delete cascade,
    access_level access_level default 'viewer'::access_level not null
);

-- Sessions

create table if not exists sessions
(
    user_id             integer                                             not null
        constraint sessions_users_id_fk
            references users
            on delete cascade,
    hash                varchar(32) default md5((random())::text)           not null,
    ip_address          inet                                                not null,
    user_agent          varchar(256)                                        not null,
    created_at          timestamp with time zone default current_timestamp  not null,
    updated_at          timestamp with time zone default current_timestamp  not null
);

create unique index if not exists sessions_hash_uindex
    on sessions (hash);

create index if not exists sessions_user_id_index
    on sessions (user_id);

create index if not exists sessions_hash_index
    on sessions (hash);

-- Stars

create table if not exists stars
(
    id          serial         not null
        constraint stars_pk
            primary key,
    stargazer   integer         not null
        constraint stars_users_id_fk
            references users
            on delete cascade,
    repo        integer         not null
        constraint stars_repositories_id_fk
            references repositories
            on delete cascade
);

create index if not exists stars_repo_index
    on stars (repo);

create index if not exists stars_stargazer_index
    on stars (stargazer);

-- SSO

-- https://stackoverflow.com/a/48382296/11494565
do $$ begin
    create type sso_provider as enum ('github', 'gitlab', 'bitbucket');
exception
    when duplicate_object then null;
end $$;

create table if not exists sso
(
    user_id     integer      not null
        constraint sso_users_id_fk
            references users
            on delete cascade,
    provider    sso_provider not null,
    provider_id varchar(64)  not null
);

create index if not exists sso_provider_id_index
    on sso (provider_id);

create index if not exists sso_provider_index
    on sso (provider);

create index if not exists sso_user_id_index
    on sso (user_id);

-- Issues

create table if not exists issues
(
    id           serial
        constraint issues_pk
            primary key,
    repo         integer                                              not null
        constraint issues_repositories_id_fk
            references repositories
            on delete cascade,
    index        integer                                              not null,
    author       integer                                              not null
        constraint issues_users_id_fk
            references users
            on delete cascade,
    title        varchar(256)                                         not null,
    labels       integer[]                default ARRAY []::integer[] not null,
    milestone    integer,
    assignees    integer[]                default ARRAY []::integer[] not null,
    closed       boolean                  default false               not null,
    confidential boolean                  default false               not null,
    locked       boolean                  default false               not null,
    created_at   timestamp with time zone default CURRENT_TIMESTAMP   not null,
    updated_at   timestamp with time zone default CURRENT_TIMESTAMP   not null
);

comment on table issues is 'Contains issues and their corresponding data; Does *not* contain the actual text content';
comment on column issues.index is 'Issue # per repository (not global instance)';

-- SSH keys

-- https://stackoverflow.com/a/48382296/11494565
do $$ begin
    create type ssh_key_type as enum (
        'ssh-rsa',
        'ecdsa-sha2-nistp256',
        'ecdsa-sha2-nistp384',
        'ecdsa-sha2-nistp521',
        'ssh-ed25519'
    );
exception
    when duplicate_object then null;
end $$;

create table if not exists ssh_keys
(
    id          serial
        constraint ssh_keys_pk
            primary key,
    owner       integer                                not null
        constraint ssh_keys_users_id_fk
            references users
            on delete cascade,
    title       varchar(64)                            not null,
    fingerprint char(47)                               not null,
    algorithm   ssh_key_type                           not null,
    key         bytea                                  not null,
    created_at  timestamp with time zone default now() not null,
    expires_at  timestamp with time zone
);

create unique index if not exists ssh_keys_fingerprint_uindex
    on ssh_keys (fingerprint);

create unique index if not exists ssh_keys_key_uindex
    on ssh_keys (key);

-- Settings

-- https://stackoverflow.com/a/48382296/11494565
do $$ begin
    create type type_constraint as enum ('boolean', 'char', 'int', 'string', 'bytes');
exception
    when duplicate_object then null;
end $$;

create table if not exists settings
(
    key varchar(64) not null
        constraint settings_pk
            primary key,
    value varchar(1024) default NULL::character varying,
    type type_constraint not null
);

create unique index if not exists settings_key_uindex
    on settings (key);

insert into settings (key, value, type) values ('domain', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('secret', md5((random())::text), 'string') on conflict do nothing;
insert into settings (key, value, type) values ('allow_registrations', null, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('repositories.base_dir', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('repositories.importing_enabled', true, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('hcaptcha.enabled', null, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('hcaptcha.site_key', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('hcaptcha.secret', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.enabled', null, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.server', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.port', null, 'int') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.tls', null, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.address', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.username', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('smtp.password', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('integrations.sentry.enabled', 'false', 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('integrations.sentry.dsn', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sessions.log_ip', true, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('sessions.log_user_agent', true, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('avatars.gravatar', true, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('avatars.dir', 'avatars', 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.github.enabled', false, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('sso.github.client_id', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.github.client_secret', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.gitlab.enabled', false, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('sso.gitlab.app_id', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.gitlab.client_secret', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.bitbucket.enabled', false, 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('sso.bitbucket.key', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('sso.bitbucket.secret', null, 'string') on conflict do nothing;
