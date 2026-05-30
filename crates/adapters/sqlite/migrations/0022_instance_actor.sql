INSERT OR IGNORE INTO users (id, email, password_hash, created_at, username)
VALUES (
    '00000000-0000-4000-8000-000000000000',
    'noreply@instance.invalid',
    '!service-actor-no-login',
    datetime('now'),
    'instance'
);
