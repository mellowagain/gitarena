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
    email      varchar(128)                                                         not null,
    password   char(96)                                                             not null,
    disabled   boolean                  default false                               not null,
    admin      boolean                  default false                               not null,
    created_at timestamp with time zone default current_timestamp                   not null,
    session    char(7)                  default substr(md5((random())::text), 0, 8) not null
);

create unique index if not exists users_username_uindex
    on users (username);

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
    license        varchar(256) default NULL::character varying
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
insert into settings (key, value, type) values ('repositories.base_dir', null, 'string');
insert into settings (key, value, type) values ('hcaptcha.enabled', null, 'boolean');
insert into settings (key, value, type) values ('hcaptcha.site_key', null, 'string');
insert into settings (key, value, type) values ('hcaptcha.secret', null, 'string');
insert into settings (key, value, type) values ('smtp.enabled', null, 'boolean');
insert into settings (key, value, type) values ('smtp.server', null, 'string');
insert into settings (key, value, type) values ('smtp.port', null, 'int');
insert into settings (key, value, type) values ('smtp.tls', null, 'boolean');
insert into settings (key, value, type) values ('smtp.email_address', null, 'string');
insert into settings (key, value, type) values ('smtp.username', null, 'string');
insert into settings (key, value, type) values ('smtp.password', null, 'string');
insert into settings (key, value, type) values ('integrations.sentry.enabled', 'false', 'boolean');
insert into settings (key, value, type) values ('integrations.sentry.dsn', null, 'string');
