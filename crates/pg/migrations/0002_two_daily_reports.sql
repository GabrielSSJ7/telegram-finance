-- Two daily summaries: yesterday's, at any hour, and today's, in the
-- evening only, so it covers most of the day.
alter table household rename column daily_report_time to today_report_time;
alter table household add column yesterday_report_time time not null default '09:00';

-- A morning report time meant "tell me about yesterday": keep it for that
-- summary and move today's to the evening default.
update household
   set yesterday_report_time = today_report_time,
       today_report_time = '21:00'
 where today_report_time < '19:00';

alter table household add constraint household_today_report_in_evening
    check (today_report_time >= '19:00');
