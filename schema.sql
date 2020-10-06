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
