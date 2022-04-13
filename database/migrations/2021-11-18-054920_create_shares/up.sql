CREATE TABLE shares (
    file_id INTEGER PRIMARY KEY NOT NULL UNIQUE,
    exp INTEGER NOT NULL,
    crt INTEGER NOT NULL,
    file_size INTEGER NOT NULL,
    user_name TEXT NOT NULL,
    file_name TEXT NOT NULL,
)