-- Your SQL goes here

CREATE TABLE documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    summary TEXT,
    keywords TEXT,
    entities TEXT,
    topics TEXT,
    uploaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
