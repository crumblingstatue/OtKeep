CREATE TABLE IF NOT EXISTS blobs (
    body BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS trees (
    root TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS tree_scripts (
    tree_id INTEGER NOT NULL,
    blob_id INTEGER NOT NULL,
    name    TEXT NOT NULL,
    desc    TEXT,
    -- The same tree cannot have multiple items with the same name
    UNIQUE(tree_id, name)
);
CREATE TABLE IF NOT EXISTS tree_files (
    tree_id INTEGER NOT NULL,
    blob_id INTEGER NOT NULL,
    name    TEXT NOT NULL,
    desc    TEXT,
    -- The same tree cannot have multiple items with the same name
    UNIQUE(tree_id, name)
);