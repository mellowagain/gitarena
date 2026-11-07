\getenv url GITARENA_URL

update settings set value = :'url' where key = 'domain.app';
update settings set value = :'url' where key = 'webauthn.origin';
update settings set value = '/repositories' where key = 'repositories.base_dir';
