create extension if not exists pgcrypto cascade;

create or replace function gen_uuidv7(ts timestamptz default clock_timestamp())
returns uuid
language plpgsql
as $$
declare
    unix_ms  bigint;
    hex      text;
    rand_a   bytea;
    rand_b   bytea;
begin
    unix_ms := (extract(epoch from ts) * 1000)::bigint;

    rand_a  := gen_random_bytes(2);
    rand_b  := gen_random_bytes(8);

    hex := lpad(to_hex(unix_ms), 12, '0');
    hex := hex || '7' || lpad(to_hex((get_byte(rand_a, 0) & x'0f'::int) << 8 | get_byte(rand_a, 1)), 3, '0');
    hex := hex || lpad(to_hex((get_byte(rand_b, 0) & x'3f'::int) | x'80'::int), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 1)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 2)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 3)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 4)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 5)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 6)), 2, '0');
    hex := hex || lpad(to_hex(get_byte(rand_b, 7)), 2, '0');

    return (
        substring(hex, 1, 8) || '-' ||
        substring(hex, 9, 4) || '-' ||
        substring(hex, 13, 4) || '-' ||
        substring(hex, 17, 4) || '-' ||
        substring(hex, 21, 12)
    )::uuid;
end;
$$;

-- Phase 1: new UUID PK columns on parent tables

alter table users add column new_id uuid;
update users set new_id = gen_uuidv7(created_at);

alter table repositories add column new_id uuid;
update repositories set new_id = gen_uuidv7();

alter table emails add column new_id uuid;
update emails set new_id = gen_uuidv7(created_at);

alter table ssh_keys add column new_id uuid;
update ssh_keys set new_id = gen_uuidv7(created_at);

alter table user_verifications add column new_id uuid;
update user_verifications set new_id = gen_uuidv7();

alter table issues add column new_id uuid;
update issues set new_id = gen_uuidv7(created_at);

-- Phase 2: new UUID FK columns in child tables

alter table emails add column new_owner uuid;
update emails set new_owner = u.new_id from users u where emails.owner = u.id;

alter table sessions add column new_user_id uuid;
update sessions set new_user_id = u.new_id from users u where sessions.user_id = u.id;

alter table user_verifications add column new_user_id uuid;
update user_verifications set new_user_id = u.new_id from users u where user_verifications.user_id = u.id;

alter table ssh_keys add column new_owner uuid;
update ssh_keys set new_owner = u.new_id from users u where ssh_keys.owner = u.id;

alter table sso add column new_user_id uuid;
update sso set new_user_id = u.new_id from users u where sso.user_id = u.id;

alter table passkeys add column new_user_id uuid;
update passkeys set new_user_id = u.new_id from users u where passkeys.user_id = u.id;

alter table webauthn_challenges add column new_user_id uuid;
update webauthn_challenges set new_user_id = u.new_id from users u where webauthn_challenges.user_id = u.id;

do $guard$
begin
    if exists (select 1 from information_schema.tables where table_schema = 'public' and table_name = 'organization_members') then
        alter table organization_members add column new_user_id uuid;
        update organization_members set new_user_id = u.new_id from users u where organization_members.user_id = u.id;
    end if;
end;
$guard$;

alter table repositories add column new_owner uuid;
update repositories set new_owner = u.new_id from users u where repositories.owner = u.id;

alter table repositories add column new_forked_from uuid;
update repositories r set new_forked_from = p.new_id from repositories p where r.forked_from = p.id;

alter table privileges add column new_user_id uuid;
update privileges set new_user_id = u.new_id from users u where privileges.user_id = u.id;

alter table privileges add column new_repo_id uuid;
update privileges set new_repo_id = r.new_id from repositories r where privileges.repo_id = r.id;

alter table stars add column new_stargazer uuid;
update stars set new_stargazer = u.new_id from users u where stars.stargazer = u.id;

alter table stars add column new_repo uuid;
update stars set new_repo = r.new_id from repositories r where stars.repo = r.id;

alter table issues add column new_repo uuid;
update issues set new_repo = r.new_id from repositories r where issues.repo = r.id;

alter table issues add column new_author uuid;
update issues set new_author = u.new_id from users u where issues.author = u.id;

-- Phase 3: drop FK constraints

alter table emails drop constraint if exists emails_users_id_fk;
alter table sessions drop constraint if exists sessions_users_id_fk;
alter table user_verifications drop constraint if exists user_verifications_user_id_fkey;
alter table ssh_keys drop constraint if exists ssh_keys_users_id_fk;
alter table repositories drop constraint if exists repositories_users_id_fk;
alter table privileges drop constraint if exists privileges_users_id_fk;
alter table privileges drop constraint if exists privileges_repositories_id_fk;
alter table stars drop constraint if exists stars_users_id_fk;
alter table stars drop constraint if exists stars_repositories_id_fk;
alter table issues drop constraint if exists issues_repositories_id_fk;
alter table issues drop constraint if exists issues_users_id_fk;
alter table sso drop constraint if exists sso_users_id_fk;
alter table passkeys drop constraint if exists passkeys_user_id_fkey;
alter table webauthn_challenges drop constraint if exists webauthn_challenges_user_id_fkey;

do $guard$
begin
    if exists (select 1 from information_schema.tables where table_schema = 'public' and table_name = 'organization_members') then
        alter table organization_members drop constraint if exists organization_members_user_id_fkey;
    end if;
end;
$guard$;

-- Phase 4: swap columns

alter table users drop constraint users_pk;
alter table users drop column id;
alter table users rename column new_id to id;
alter table users add constraint users_pk primary key (id);
alter table users alter column id set not null;
alter table users drop column created_at;
drop sequence if exists users_id_seq;

alter table repositories drop constraint repositories_pk;
alter table repositories drop column id;
alter table repositories rename column new_id to id;
alter table repositories add constraint repositories_pk primary key (id);
alter table repositories alter column id set not null;
alter table repositories drop column owner;
alter table repositories rename column new_owner to owner;
alter table repositories alter column owner set not null;
alter table repositories drop column forked_from;
alter table repositories rename column new_forked_from to forked_from;
drop sequence if exists repositories_id_seq;

alter table emails drop constraint emails_pk;
alter table emails drop column id;
alter table emails rename column new_id to id;
alter table emails add constraint emails_pk primary key (id);
alter table emails alter column id set not null;
alter table emails drop column owner;
alter table emails rename column new_owner to owner;
alter table emails alter column owner set not null;
alter table emails drop column created_at;
drop sequence if exists emails_id_seq;

alter table sessions drop column user_id;
alter table sessions rename column new_user_id to user_id;
alter table sessions alter column user_id set not null;
alter table sessions drop column created_at;

alter table user_verifications drop constraint user_verifications_pk;
alter table user_verifications drop column id;
alter table user_verifications rename column new_id to id;
alter table user_verifications add constraint user_verifications_pk primary key (id);
alter table user_verifications alter column id set not null;
alter table user_verifications drop column user_id;
alter table user_verifications rename column new_user_id to user_id;
alter table user_verifications alter column user_id set not null;
drop sequence if exists user_verifications_id_seq;

alter table ssh_keys drop constraint ssh_keys_pk;
alter table ssh_keys drop column id;
alter table ssh_keys rename column new_id to id;
alter table ssh_keys add constraint ssh_keys_pk primary key (id);
alter table ssh_keys alter column id set not null;
alter table ssh_keys drop column owner;
alter table ssh_keys rename column new_owner to owner;
alter table ssh_keys alter column owner set not null;
alter table ssh_keys drop column created_at;
drop sequence if exists ssh_keys_id_seq;

alter table issues drop constraint issues_pk;
alter table issues drop column id;
alter table issues rename column new_id to id;
alter table issues add constraint issues_pk primary key (id);
alter table issues alter column id set not null;
alter table issues drop column repo;
alter table issues rename column new_repo to repo;
alter table issues alter column repo set not null;
alter table issues drop column author;
alter table issues rename column new_author to author;
alter table issues alter column author set not null;
alter table issues drop column created_at;
drop sequence if exists issues_id_seq;

alter table privileges drop constraint privileges_pk;
alter table privileges drop column id;
alter table privileges drop column user_id;
alter table privileges rename column new_user_id to user_id;
alter table privileges alter column user_id set not null;
alter table privileges drop column repo_id;
alter table privileges rename column new_repo_id to repo_id;
alter table privileges alter column repo_id set not null;
alter table privileges add constraint privileges_pk primary key (user_id, repo_id);
drop sequence if exists privileges_id_seq;

alter table stars drop constraint stars_pk;
alter table stars drop column id;
alter table stars drop column stargazer;
alter table stars rename column new_stargazer to stargazer;
alter table stars alter column stargazer set not null;
alter table stars drop column repo;
alter table stars rename column new_repo to repo;
alter table stars alter column repo set not null;
alter table stars add constraint stars_pk primary key (stargazer, repo);
drop sequence if exists stars_id_seq;

alter table sso drop column user_id;
alter table sso rename column new_user_id to user_id;
alter table sso alter column user_id set not null;

alter table passkeys drop column user_id;
alter table passkeys rename column new_user_id to user_id;
alter table passkeys alter column user_id set not null;

alter table webauthn_challenges drop column user_id;
alter table webauthn_challenges rename column new_user_id to user_id;

do $guard$
begin
    if exists (select 1 from information_schema.tables where table_schema = 'public' and table_name = 'organization_members') then
        alter table organization_members drop column user_id;
        alter table organization_members rename column new_user_id to user_id;
    end if;
end;
$guard$;

-- Phase 5: recreate FK constraints

alter table emails add constraint emails_users_id_fk foreign key (owner) references users (id) on delete cascade;
alter table sessions add constraint sessions_users_id_fk foreign key (user_id) references users (id) on delete cascade;
alter table user_verifications add constraint user_verifications_user_id_fkey foreign key (user_id) references users (id) on delete cascade;
alter table ssh_keys add constraint ssh_keys_users_id_fk foreign key (owner) references users (id) on delete cascade;
alter table repositories add constraint repositories_users_id_fk foreign key (owner) references users (id) on delete cascade;
alter table repositories add constraint repositories_forked_from_fk foreign key (forked_from) references repositories (id) on delete set null;
alter table privileges add constraint privileges_users_id_fk foreign key (user_id) references users (id) on delete cascade;
alter table privileges add constraint privileges_repositories_id_fk foreign key (repo_id) references repositories (id) on delete cascade;
alter table stars add constraint stars_users_id_fk foreign key (stargazer) references users (id) on delete cascade;
alter table stars add constraint stars_repositories_id_fk foreign key (repo) references repositories (id) on delete cascade;
alter table issues add constraint issues_repositories_id_fk foreign key (repo) references repositories (id) on delete cascade;
alter table issues add constraint issues_users_id_fk foreign key (author) references users (id) on delete cascade;
alter table sso add constraint sso_users_id_fk foreign key (user_id) references users (id) on delete cascade;
alter table passkeys add constraint passkeys_user_id_fkey foreign key (user_id) references users (id) on delete cascade;
alter table webauthn_challenges add constraint webauthn_challenges_user_id_fkey foreign key (user_id) references users (id) on delete cascade;

do $guard$
begin
    if exists (select 1 from information_schema.tables where table_schema = 'public' and table_name = 'organization_members') then
        alter table organization_members add constraint organization_members_user_id_fkey foreign key (user_id) references users (id) on delete cascade;
    end if;
end;
$guard$;

drop function gen_uuidv7;
