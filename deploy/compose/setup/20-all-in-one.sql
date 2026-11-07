\getenv meili_key MEILI_MASTER_KEY
\getenv s3_url S3_URL
\getenv s3_access_key RUSTFS_ACCESS_KEY
\getenv s3_secret_key RUSTFS_SECRET_KEY

update settings set value = 'true' where key = 'meilisearch.enabled';
update settings set value = 'http://meilisearch:7700' where key = 'meilisearch.url';
update settings set value = :'meili_key' where key = 'meilisearch.key';

update settings set value = 'true' where key = 'zoekt.enabled';
update settings set value = 'http://zoekt:6070' where key = 'zoekt.address';
update settings set value = '/data/index' where key = 'zoekt.index_dir';
update settings set value = '/usr/local/bin/zoekt-git-index' where key = 'zoekt.index_binary_path';

update settings set value = 'true' where key = 's3.enabled';
update settings set value = :'s3_url' where key = 's3.endpoint';
update settings set value = 'gitarena' where key = 's3.bucket';
update settings set value = 'us-east-1' where key = 's3.region';
update settings set value = 'true' where key = 's3.force_path_style';
update settings set value = :'s3_access_key' where key = 's3.access_key_id';
update settings set value = :'s3_secret_key' where key = 's3.secret_access_key';
