create table if not exists commit_contributions (
    user_id     uuid not null references users(id) on delete cascade,
    repo_id     uuid not null references repositories(id) on delete cascade,
    commit_sha  char(40) not null,
    author_date date not null,
    primary key (user_id, repo_id, commit_sha)
);

create index if not exists idx_commit_contributions_lookup
    on commit_contributions (user_id, author_date);

insert into settings (key, value, type)
values ('contributions.gitarena_version', '0', 'int')
on conflict do nothing;
