insert into settings (key, value, type)
values ('domain.backend', null, 'string')
on conflict do nothing;
