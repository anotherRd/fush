CREATE TABLE IF NOT EXISTS nodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    address TETXT,
    auth_type TEXT,
    node_type TEXT,
    parent_id INTEGER,
    FOREIGN KEY (parent_id) REFERENCES nodes(id) ON DELETE CASCADE
)