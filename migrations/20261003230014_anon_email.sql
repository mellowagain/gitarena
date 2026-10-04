update settings set key = 'domain.app' where key = 'domain';

insert into settings (key, value, type) select
    'domain.user_email',
    replace(replace(value, 'https://', ''), 'http://', ''),
    'string'
from settings where key = 'domain.app'
on conflict do nothing;
