insert into settings (key, value, type) values ('webauthn.origin', null, 'string') on conflict do nothing;

update settings set value = 'false' where key = 'smtp.enabled' and value is null;
