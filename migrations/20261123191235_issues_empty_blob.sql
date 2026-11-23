insert into settings (key, value, type)
values ('issues.gitarena_version', '0', 'int')
on conflict do nothing;
