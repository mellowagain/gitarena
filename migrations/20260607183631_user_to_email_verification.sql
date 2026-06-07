alter table user_verifications add column if not exists email text;

update user_verifications uv set email = coalesce(
    (select e.email from emails e where e.owner = uv.user_id and e.verified_at is null order by e.id limit 1),
    ''
);

alter table user_verifications alter column email set not null;
