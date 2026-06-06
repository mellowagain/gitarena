create table if not exists releases
(
    id          uuid    not null primary key,
    repo_id     uuid    not null references repositories on delete cascade,

    title       text    not null,
    description text,

    author      uuid    references users on delete set null,

    tag         text    not null,
    pre_release boolean not null
);

create index if not exists releases_repo_id_index on releases (repo_id);

do
$$
    begin
        create type asset_os as enum ('linux', 'windows', 'macos', 'freebsd', 'openbsd', 'netbsd', 'android', 'ios', 'unknown');
    exception
        when duplicate_object then null;
    end
$$;

do
$$
    begin
        create type asset_arch as enum ('x86_64', 'i686', 'aarch64', 'armv7', 'armv6', 'riscv64', 'loongarch64', 'powerpc64', 's390x', 'wasm32', 'universal', 'unknown');
    exception
        when duplicate_object then null;
    end
$$;

do
$$
    begin
        create type asset_libc as enum ('gnu', 'musl', 'msvc', 'mingw', 'bionic', 'unknown');
    exception
        when duplicate_object then null;
    end
$$;

do
$$
    begin
        create type asset_kind as enum ('binary', 'installer', 'library', 'source', 'sbom', 'other');
    exception
        when duplicate_object then null;
    end
$$;

create table if not exists release_assets
(
    id          uuid    not null primary key,
    release_id  uuid    not null references releases on delete cascade,

    name        text    not null,
    size        bigint  not null,
    hash        text    not null,

    available   boolean not null    default false,
    downloads   bigint  not null    default 0,

    os          asset_os            default null,
    arch        asset_arch          default null,
    libc        asset_libc          default null,
    kind        asset_kind          default null
);

create index if not exists release_assets_release_id_index on release_assets (release_id);

insert into settings (key, value, type) values ('s3.enabled', 'false', 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('s3.bucket', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('s3.region', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('s3.endpoint', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('s3.access_key_id', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('s3.secret_access_key', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('s3.force_path_style', 'false', 'boolean') on conflict do nothing;
