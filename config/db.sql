CREATE TABLE IF NOT EXISTS shares
(
    id BIGSERIAL PRIMARY KEY NOT NULL,
    uuid UUID NOT NULL,
    created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
    expires timestamp with time zone DEFAULT (now() at time zone 'utc'),
    usr text NOT NULL,
    website BOOLEAN NOT NULL,
    wget BOOLEAN NOT NULL,
    name text NOT NULL,
    size BIGINT NOT NULL,
    file_type text NOT NULL
);