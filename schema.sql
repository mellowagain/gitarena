create table users
(
    id             serial                not null,
    username       varchar(32)           not null,
    email          varchar(128)          not null,
    password       char(96)              not null,
    salt           char(16)              not null,
    email_verified boolean default false not null
);

create unique index users_username_uindex
	on users (username);

create table user_verifications
(
    id      serial                   not null
        constraint user_verifications_pk
            primary key,
    user_id integer                  not null,
    hash    char(32)                 not null,
    expires timestamp with time zone not null
);

create unique index user_verifications_hash_uindex
    on user_verifications (hash);

create unique index user_verifications_user_id_uindex
    on user_verifications (user_id);
