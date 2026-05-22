insert into settings (key, value, type) values ('meilisearch.enabled', 'false', 'boolean') on conflict do nothing;
insert into settings (key, value, type) values ('meilisearch.url', 'http://127.0.0.1:7700', 'string') on conflict do nothing;
insert into settings (key, value, type) values ('meilisearch.key', '', 'string') on conflict do nothing;
