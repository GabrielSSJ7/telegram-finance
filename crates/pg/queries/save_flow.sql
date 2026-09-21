insert into chat_flows (chat_id, user_id, flow, prompt_message_id, draft_id, expires_at)
values ($1, $2, $3, $4, $5, $6)
on conflict (chat_id, user_id) do update set
    flow = excluded.flow,
    prompt_message_id = excluded.prompt_message_id,
    draft_id = excluded.draft_id,
    expires_at = excluded.expires_at
