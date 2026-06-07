do
$$
    begin
        create type event_class as enum ('security', 'activity', 'system');
    exception
        when duplicate_object then null;
    end
$$;

create table if not exists events
(
    id uuid primary key,
    trace_id uuid,

    actor_id uuid not null,
    ip_address inet,
    user_agent text,

    -- idk if set null is right
    subject_id_user uuid references users on delete set null,
    subject_id_org uuid references organizations on delete set null,
    subject_id_repo uuid references repositories on delete set null,

    class event_class not null,
    type text not null,
    payload jsonb not null default '{}'
);

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

with ghost_id as (
    insert into users (id, username, password, disabled, admin)
    values (gen_uuidv7(), 'gitarena', '', false, true)
    returning id
)

insert into emails (id, owner, email, "primary", "commit", notification, public, verified_at)
select gen_uuidv7(), id, 'root@gitarena.local', true, true, true, false, now()
from ghost_id;

drop function gen_uuidv7;
