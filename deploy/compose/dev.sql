update settings set value = 'http://localhost:8080' where key = 'domain.app';
update settings set value = 'http://localhost:8321' where key = 'webauthn.origin';
update settings set value = 'repositories' where key = 'repositories.base_dir';

update settings set value = 'true' where key = 'meilisearch.enabled';
update settings set value = 'http://127.0.0.1:7700' where key = 'meilisearch.url';
update settings set value = 'gitarena-meilisearch-master-key' where key = 'meilisearch.key';

update settings set value = 'true' where key = 'zoekt.enabled';
update settings set value = 'http://127.0.0.1:6070' where key = 'zoekt.address';
update settings set value = '~/.local/share/gitarena/zoekt-index' where key = 'zoekt.index_dir';

update settings set value = 'true' where key = 's3.enabled';
update settings set value = 'http://localhost:9000' where key = 's3.endpoint';
update settings set value = 'gitarena' where key = 's3.bucket';
update settings set value = 'us-east-1' where key = 's3.region';
update settings set value = 'true' where key = 's3.force_path_style';
update settings set value = 'rustfsadmin' where key = 's3.access_key_id';
update settings set value = 'rustfsadmin' where key = 's3.secret_access_key';

update settings set value = 'true' where key = 'smtp.enabled';
update settings set value = 'localhost' where key = 'smtp.server';
update settings set value = '1025' where key = 'smtp.port';
update settings set value = 'false' where key = 'smtp.tls';
update settings set value = 'gitarena' where key = 'smtp.username';
update settings set value = 'gitarena' where key = 'smtp.password';
update settings set value = 'gitarena@localhost' where key = 'smtp.address';
