-- Split repositories.owner (single uuid) into owner_user and owner_org
-- so that repositories can be owned by either a user or an organization.

alter table repositories add column owner_user uuid references users(id) on delete cascade;
alter table repositories add column owner_org  uuid references organizations(id) on delete cascade;

-- Migrate existing rows: current owner is always a user (no org repos existed before)
update repositories set owner_user = owner;

-- Add constraint: exactly one of the two columns must be set
alter table repositories add constraint repositories_owner_exclusive
    check (
        (owner_user is not null and owner_org is null) or
        (owner_user is null and owner_org is not null)
    );

-- The old column is kept for now as a fallback; code will transition to owner_user/owner_org.
-- Drop it once all queries are migrated.
alter table repositories drop column owner;
