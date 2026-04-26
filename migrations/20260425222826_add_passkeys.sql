create table passkeys (
    id          uuid primary key,
    user_id     int not null references users (id) on delete cascade,
    name        varchar(64) not null,
    credential  jsonb not null,
    unique (user_id, name)
);

create type webauthn_challenge_type as enum ('registration', 'authentication');

create table webauthn_challenges (
    id          uuid primary key,
    user_id     int references users (id) on delete cascade,
    state       jsonb not null,
    type        webauthn_challenge_type not null,
    expires_at  timestamptz not null
);

create index idx_webauthn_challenges_expires on webauthn_challenges (expires_at);
