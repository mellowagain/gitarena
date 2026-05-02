insert into settings (key, value, type) values ('ssh.enabled', 'true', 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('ssh.port', '2222', 'int') on conflict do nothing;
insert into settings (key, value, type) values ('ssh.private_key', null, 'string') on conflict do nothing;
insert into settings (key, value, type) values ('ssh.auth_rejection_time_seconds', '5', 'int') on conflict do nothing;

delete from ssh_keys;
alter table ssh_keys alter column fingerprint type char(43);
