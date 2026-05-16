insert into settings (key, value, type) values ('zoekt.enabled', 'false', 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('zoekt.address', 'http://127.0.0.1:6070', 'string') on conflict do nothing;
insert into settings (key, value, type) values ('zoekt.index_binary_path', '~/go/bin/zoekt-git-index', 'string') on conflict do nothing;
insert into settings (key, value, type) values ('zoekt.index_dir', '/data/index', 'string') on conflict do nothing;
insert into settings (key, value, type) values ('zoekt.gitarena_version', '0', 'int') on conflict do nothing;

alter table repositories add column if not exists zoekt_id serial not null;
create unique index if not exists idx_uniq_repos_zoekt_id on repositories (zoekt_id);

create index if not exists idx_repos_owner_user on repositories (owner_user);
create index if not exists idx_repos_owner_org on repositories (owner_org);

create index if not exists idx_org_members_user_id on organization_members (user_id);

create index if not exists idx_privileges_user_id on privileges (user_id);

create index if not exists idx_repos_zoekt_id_public on repositories (zoekt_id) where visibility in ('public', 'internal');
