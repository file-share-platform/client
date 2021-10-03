CREATE TABLE IF NOT EXISTS shares
(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    public_id text NOT NULL,
    created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
    expires timestamp with time zone DEFAULT (now() at time zone 'utc'),
    usr text NOT NULL,
    website BOOLEAN NOT NULL,
    wget BOOLEAN NOT NULL,
    name text NOT NULL,
    size BIGINT NOT NULL,
    file_type text NOT NULL
);


-- INSERT INTO shares (uuid, usr, website, wget, name, size, file_type)
-- VALUES ('A0EEBC99-9C0B-4EF8-BB6D-6BB9BD380A11', 'josiah', TRUE, TRUE, 'my_file', 5, 'txt')
-- RETURNING *;