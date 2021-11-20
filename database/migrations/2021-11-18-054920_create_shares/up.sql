CREATE TABLE shares (
    id BLOB NOT NULL,
    user TEXT NOT NULL,
    exp DATETIME NOT NULL,
    crt DATETIME NOT NULL,
    name TEXT NOT NULL,
    size INTEGER NOT NULL,
    ext TEXT NOT NULL,
    PRIMARY KEY (id)
)