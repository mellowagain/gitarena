create table if not exists sso_state
(
    csrf_token text                      not null
        constraint sso_state_pk
            primary key,
    provider   text                      not null,
    state      jsonb                     not null,
    created_at timestamptz default now() not null
);

alter table if exists sso alter column provider type text using provider::text;
alter table if exists sso rename to sso_connections;

drop type if exists sso_provider cascade;

do $$ begin
    if exists (select 1 from settings where key = 'sso.github.enabled' and value = 'true') then
        update settings set key = 'sso.oauth2.github.client_id' where key = 'sso.github.client_id';
        update settings set key = 'sso.oauth2.github.client_secret' where key = 'sso.github.client_secret';
    end if;
end $$;

delete from settings where key = 'sso.github.enabled';
delete from settings where key = 'sso.github.client_id';
delete from settings where key = 'sso.github.client_secret';

do $$ begin
    if exists (select 1 from settings where key = 'sso.gitlab.enabled' and value = 'true') then
        update settings set key = 'sso.oauth2.gitlab.client_id' where key = 'sso.gitlab.app_id';
        update settings set key = 'sso.oauth2.gitlab.client_secret' where key = 'sso.gitlab.client_secret';
    end if;
end $$;

delete from settings where key = 'sso.gitlab.enabled';
delete from settings where key = 'sso.gitlab.app_id';
delete from settings where key = 'sso.gitlab.client_secret';

do $$ begin
    if exists (select 1 from settings where key = 'sso.bitbucket.enabled' and value = 'true') then
        update settings set key = 'sso.oauth2.bitbucket.client_id' where key = 'sso.bitbucket.key';
        update settings set key = 'sso.oauth2.bitbucket.client_secret' where key = 'sso.bitbucket.secret';
    end if;
end $$;

delete from settings where key = 'sso.bitbucket.enabled';
delete from settings where key = 'sso.bitbucket.key';
delete from settings where key = 'sso.bitbucket.secret';
