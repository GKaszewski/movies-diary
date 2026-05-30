INSERT INTO users (id, username, email, password_hash, created_at, role)
VALUES (
    '00000000-0000-4000-8000-000000000000',
    'instance',
    'noreply@instance.invalid',
    '!service-actor-no-login',
    NOW(),
    'standard'
)
ON CONFLICT (id) DO NOTHING;
