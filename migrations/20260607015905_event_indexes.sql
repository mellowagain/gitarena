create index if not exists idx_events_actor on events (actor_id, id desc);
create index if not exists idx_events_class on events (class, id desc);
create index if not exists idx_events_subject_repo on events (subject_id_repo, id desc);
create index if not exists idx_events_subject_org on events (subject_id_org, id desc);
create index if not exists idx_events_subject_user on events (subject_id_user, id desc);
