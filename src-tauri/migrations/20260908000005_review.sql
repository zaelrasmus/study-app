-- Review scheduling, and the third origin gets its real name.

-- Scheduling state only. There is no front, no back and no content here,
-- because an atomic note *is* the card: front is the title, back is the body.
-- A card therefore cannot drift from its source, and editing a card means
-- editing the note.
--
-- Rows are created lazily on the first review, so a note that has never been
-- reproduced from memory simply has no scheduling state to carry.
CREATE TABLE review_state (
    note_id          TEXT PRIMARY KEY REFERENCES notes (id) ON DELETE CASCADE,
    due_at           TEXT    NOT NULL,
    interval_days    REAL    NOT NULL DEFAULT 0,
    ease             REAL    NOT NULL DEFAULT 2.5,
    reps             INTEGER NOT NULL DEFAULT 0,
    lapses           INTEGER NOT NULL DEFAULT 0,
    last_reviewed_at TEXT,
    created_at       TEXT    NOT NULL,
    updated_at       TEXT    NOT NULL
);

CREATE INDEX idx_review_state_due ON review_state (due_at);

-- `reading` was the wrong word for the lowest-priority origin: it is not about
-- reading, it is about anything you captured without the system observing a
-- gap. SQLite cannot alter a CHECK constraint, so the table is rebuilt --
-- nothing references `questions`, which makes this safe.
CREATE TABLE questions_new (
    id     TEXT NOT NULL PRIMARY KEY,
    text   TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open'
           CHECK (status IN ('open', 'investigating', 'resolved', 'abandoned', 'dissolved')),
    origin TEXT NOT NULL
           CHECK (origin IN ('recall_gap', 'review_failure', 'capture')),

    topic_id       TEXT REFERENCES topics (id) ON DELETE SET NULL,
    source_note_id TEXT REFERENCES notes  (id) ON DELETE SET NULL,
    answer_note_id TEXT REFERENCES notes  (id) ON DELETE SET NULL,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (status <> 'resolved' OR answer_note_id IS NOT NULL)
);

INSERT INTO questions_new (id, text, status, origin, topic_id, source_note_id,
                           answer_note_id, created_at, updated_at)
     SELECT id, text, status,
            CASE origin WHEN 'reading' THEN 'capture' ELSE origin END,
            topic_id, source_note_id, answer_note_id, created_at, updated_at
       FROM questions;

DROP TABLE questions;
ALTER TABLE questions_new RENAME TO questions;

CREATE INDEX idx_questions_status ON questions (status, origin);
CREATE INDEX idx_questions_topic  ON questions (topic_id);
