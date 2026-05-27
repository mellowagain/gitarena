alter table repositories add column if not exists archived_at timestamptz null;

update repositories set archived_at = now() where archived = true and archived_at is null;

alter table repositories drop column if exists archived;
