drop table if exists issues;

alter table repositories add column if not exists next_issue_index integer not null default 0;

create table if not exists labels
(
    id          uuid                   not null primary key,
    repo_id     uuid                   not null references repositories (id) on delete cascade,
    name        text                   not null,
    color       text default '#888888' not null,
    description text
);

create unique index if not exists labels_repo_name_uindex
    on labels (repo_id, name);

create index if not exists labels_repo_id_index
    on labels (repo_id);

create table if not exists milestones
(
    id          uuid                                               not null primary key,
    repo_id     uuid                                               not null references repositories (id) on delete cascade,
    title       text                                               not null,
    description text,
    due_date    timestamp with time zone,
    closed      boolean default false                              not null,
    created_at  timestamp with time zone default current_timestamp not null
);

create index if not exists milestones_repo_index
    on milestones (repo_id);

do $$
    begin
        create type issue_status as enum ('open', 'in_progress', 'completed', 'not_planned');
    exception
        when duplicate_object then null;
    end;
$$;

create table if not exists issue_cache
(
    id            uuid                                               not null   primary key,
    repo_id       uuid                                               not null   references repositories (id) on delete cascade,
    git_bug_id    text                                               not null,
    "index"       integer                                            not null,
    author_id     uuid                                               not null   references users (id) on delete cascade,
    title         text                                               not null,
    body          text    default ''                                 not null,
    status        issue_status default 'open'                        not null,
    labels        text[]  default '{}'                               not null,
    confidential  boolean default false                              not null,
    locked        boolean default false                              not null,
    milestone_id  uuid                                                          references milestones (id) on delete set null,
    assignees     uuid[]  default '{}'                               not null,
    priority      text    default 'none'                             not null   check (priority in ('none', 'low', 'medium', 'high', 'urgent')),
    updated_at    timestamp with time zone default current_timestamp not null
);

create unique index if not exists issue_cache_repo_index_uindex
    on issue_cache (repo_id, "index");

create unique index if not exists issue_cache_repo_git_bug_uindex
    on issue_cache (repo_id, git_bug_id);

create index if not exists issue_cache_author_index
    on issue_cache (author_id);

create table if not exists issue_comment_cache
(
    id        uuid                     not null primary key,
    op_id     text                     not null,
    issue_id  uuid                     not null references issue_cache (id) on delete cascade,
    author_id uuid                     not null references users (id) on delete cascade,
    body      text                     not null,
    edited_at timestamp with time zone
);

create index if not exists issue_comment_cache_issue_index
    on issue_comment_cache (issue_id);

create table if not exists reactions
(
    id         uuid not null    primary key,
    user_id    uuid not null    references users (id) on delete cascade,
    emoji      text not null,
    issue_id   uuid             references issue_cache (id) on delete cascade,
    comment_id uuid             references issue_comment_cache (id) on delete cascade,

    constraint reactions_target_check   check ((issue_id is null) != (comment_id is null)),
    constraint reactions_issue_unique   unique (user_id, emoji, issue_id),
    constraint reactions_comment_unique unique (user_id, emoji, comment_id)
);

create index if not exists reactions_issue_id_index
    on reactions (issue_id);

create index if not exists reactions_comment_id_index
    on reactions (comment_id);
