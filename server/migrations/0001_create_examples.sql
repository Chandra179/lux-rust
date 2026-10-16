CREATE TABLE examples (
    id         text PRIMARY KEY,
    name       text NOT NULL,
    created_at datetime NOT NULL DEFAULT (datetime('now'))
);
