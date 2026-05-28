create table if not exists organizations (
    id uuid primary key,
    name text unique not null,
    description text not null default ''
);

do $$
    begin
        create type org_role as enum ('owner', 'admin', 'member');
    exception
        when duplicate_object then null;
    end;
$$;

create table if not exists organization_members (
    org_id uuid not null references organizations(id) on delete cascade,
    user_id uuid not null references users(id) on delete cascade,
    role org_role not null default 'member',
    primary key (org_id, user_id)
);
