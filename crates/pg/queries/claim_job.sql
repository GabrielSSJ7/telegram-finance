-- Claims (job, run_date). A second claim succeeds only for a failed run
-- under the attempt limit, or a run that started over ten minutes ago and
-- never finished (the process died). Returns a row when claimed.
insert into job_runs (job, run_date, status, attempts, updated_at)
values ($1, $2, 'running', 1, $3)
on conflict (job, run_date) do update
    set status = 'running', attempts = job_runs.attempts + 1, updated_at = excluded.updated_at
    where job_runs.status <> 'succeeded'
      and job_runs.attempts < $4
      and (job_runs.status = 'failed' or job_runs.updated_at < excluded.updated_at - interval '10 minutes')
returning job
