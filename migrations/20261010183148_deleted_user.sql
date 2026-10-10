-- constraint names differ between instances so look them up
do
$$
    declare
        fk record;
    begin
        for fk in
            select conrelid::regclass as tbl, conname
            from pg_constraint
            where contype = 'f'
              and confrelid = 'users'::regclass
              and conrelid in ('issue_cache'::regclass, 'issue_comment_cache'::regclass)
        loop
            execute format('alter table %s drop constraint %I', fk.tbl, fk.conname);
        end loop;
    end
$$;

alter table issue_cache add constraint issue_cache_author_id_fkey foreign key (author_id) references users (id);
alter table issue_comment_cache add constraint issue_comment_cache_author_id_fkey foreign key (author_id) references users (id);

insert into users (id, username, password, disabled, admin)
values ('01a12715-e373-78d7-ab44-172357bcbe68', 'deleted-user', '', true, false)
on conflict do nothing;
