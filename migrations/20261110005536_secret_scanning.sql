insert into settings (key, value, type)
values ('secret_scanning.enabled', 'true', 'boolean')
on conflict do nothing;
