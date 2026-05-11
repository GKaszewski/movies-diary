CREATE TABLE blocked_actors (
    local_user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    remote_actor_url TEXT NOT NULL,
    blocked_at       TEXT NOT NULL,
    PRIMARY KEY (local_user_id, remote_actor_url)
);
