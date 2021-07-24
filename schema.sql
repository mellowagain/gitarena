create table if not exists users
(
    id         serial                                                               not null
        constraint users_pk
            primary key,
    username   varchar(32)                                                          not null,
    email      varchar(128)                                                         not null,
    password   char(96)                                                             not null,
    disabled   boolean                  default false                               not null,
    created_at timestamp with time zone default CURRENT_TIMESTAMP                   not null,
    session    char(7)                  default substr(md5((random())::text), 0, 8) not null
);

create unique index if not exists users_username_uindex
    on users (username);

create table if not exists user_verifications
(
    id      serial                   not null
        constraint user_verifications_pk
            primary key,
    user_id integer                  not null,
    hash    char(32)                 not null,
    expires timestamp with time zone not null
);

create unique index if not exists user_verifications_hash_uindex
    on user_verifications (hash);

create unique index if not exists user_verifications_user_id_uindex
    on user_verifications (user_id);

create table if not exists repositories
(
    id          serial       not null
        constraint repositories_pk
            primary key,
    owner       integer      not null
        constraint repositories_users_id_fk
            references users (id)
            on delete cascade,
    name        varchar(32)  not null,
    description varchar(256) not null
);
