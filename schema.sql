-- This file will be executed when GitArena detects an empty database without our schema (basically when GitArena runs for the first time)
-- It gets included into GitArena using the include_str! macro, so changes require a recompilation
-- STYLE: SQL keywords are always lowercase. Names follow snake_case. Every statement needs to end in an semicolon.
-- TODO: Use sqlx migrations instead to replace this in the future

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

create table emails
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

create unique index emails_email_uindex
    on emails (email);

create index emails_owner_index
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

create type repo_visibility as enum ('public', 'internal', 'private');

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

create type access_level as enum ('viewer', 'supporter', 'coder', 'manager', 'admin');

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

create table sessions
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

create unique index sessions_hash_uindex
    on sessions (hash);

create index sessions_user_id_index
    on sessions (user_id);

create index sessions_hash_index
    on sessions (hash);

-- Stars

create table stars
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

create index stars_repo_index
    on stars (repo);

create index stars_stargazer_index
    on stars (stargazer);

-- SSO

create type sso_provider as enum ('github', 'gitlab', 'bitbucket');

create table sso
(
    user_id     integer      not null
        constraint sso_users_id_fk
            references users
            on delete cascade,
    provider    sso_provider not null,
    provider_id varchar(64)  not null
);

create index sso_provider_id_index
    on sso (provider_id);

create index sso_provider_index
    on sso (provider);

create index sso_user_id_index
    on sso (user_id);

-- Issues

create table issues
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

-- Settings
-- CONTRIBUTING: This table always needs to be the last in this file. Please add new tables above this section.

create type type_constraint as enum ('boolean', 'char', 'int', 'string', 'bytes');

create table settings
(
    key varchar(64) not null
        constraint settings_pk
            primary key,
    value varchar(1024) default NULL::character varying,
    type type_constraint not null
);

create unique index settings_key_uindex
    on settings (key);

-- CONTRIBUTING: If adding new settings, please add key, default value (or null) and type below

insert into settings (key, value, type) values ('domain', null, 'string');
insert into settings (key, value, type) values ('secret', md5((random())::text), 'string');
insert into settings (key, value, type) values ('allow_registrations', null, 'boolean');
insert into settings (key, value, type) values ('repositories.base_dir', null, 'string');
insert into settings (key, value, type) values ('hcaptcha.enabled', null, 'boolean');
insert into settings (key, value, type) values ('hcaptcha.site_key', null, 'string');
insert into settings (key, value, type) values ('hcaptcha.secret', null, 'string');
insert into settings (key, value, type) values ('smtp.enabled', null, 'boolean');
insert into settings (key, value, type) values ('smtp.server', null, 'string');
insert into settings (key, value, type) values ('smtp.port', null, 'int');
insert into settings (key, value, type) values ('smtp.tls', null, 'boolean');
insert into settings (key, value, type) values ('smtp.address', null, 'string');
insert into settings (key, value, type) values ('smtp.username', null, 'string');
insert into settings (key, value, type) values ('smtp.password', null, 'string');
insert into settings (key, value, type) values ('integrations.sentry.enabled', 'false', 'boolean');
insert into settings (key, value, type) values ('integrations.sentry.dsn', null, 'string');
insert into settings (key, value, type) values ('sessions.log_ip', true, 'boolean');
insert into settings (key, value, type) values ('sessions.log_user_agent', true, 'boolean');
insert into settings (key, value, type) values ('avatars.gravatar', true, 'boolean');
insert into settings (key, value, type) values ('avatars.dir', 'avatars', 'string');
insert into settings (key, value, type) values ('sso.github.enabled', false, 'boolean');
insert into settings (key, value, type) values ('sso.github.client_id', null, 'string');
insert into settings (key, value, type) values ('sso.github.client_secret', null, 'string');
insert into settings (key, value, type) values ('sso.gitlab.enabled', false, 'boolean');
insert into settings (key, value, type) values ('sso.gitlab.app_id', null, 'string');
insert into settings (key, value, type) values ('sso.gitlab.client_secret', null, 'string');
insert into settings (key, value, type) values ('sso.bitbucket.enabled', false, 'boolean');
insert into settings (key, value, type) values ('sso.bitbucket.key', null, 'string');
insert into settings (key, value, type) values ('sso.bitbucket.secret', null, 'string');
