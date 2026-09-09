-- A question you can put on a board.
--
-- Questions are doubts, and doubts surface while you are writing messily about
-- something — which means the thing they are about is usually already on a
-- board. Until now a question could only ever be a row in a queue, so the doubt
-- and the cards that might answer it lived in different places and you had to
-- hold the connection in your head.
--
-- Placing one is also how a stray question *acquires* a board: dropping it on a
-- canvas attaches it, which is the same rule as notes — placement is membership.

CREATE TABLE canvas_nodes_new (
    id       TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,
    kind     TEXT NOT NULL CHECK (
        kind IN ('note', 'ink', 'image', 'pdf', 'topic', 'text', 'link', 'question')
    ),

    x      REAL NOT NULL DEFAULT 0,
    y      REAL NOT NULL DEFAULT 0,
    width  REAL,
    height REAL,
    z      INTEGER NOT NULL DEFAULT 0,

    note_id         TEXT REFERENCES notes     (id) ON DELETE CASCADE,
    target_topic_id TEXT REFERENCES topics    (id) ON DELETE CASCADE,
    asset_id        TEXT REFERENCES assets    (id) ON DELETE CASCADE,
    question_id     TEXT REFERENCES questions (id) ON DELETE CASCADE,
    ink             TEXT,
    text            TEXT,
    url             TEXT,

    color TEXT,

    frame_id       TEXT REFERENCES frames (id) ON DELETE SET NULL,
    order_in_frame INTEGER,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (
        (kind = 'note'            AND note_id         IS NOT NULL) OR
        (kind = 'topic'           AND target_topic_id IS NOT NULL) OR
        (kind IN ('image', 'pdf') AND asset_id        IS NOT NULL) OR
        (kind = 'question'        AND question_id     IS NOT NULL) OR
        (kind = 'ink'             AND ink             IS NOT NULL) OR
        (kind = 'text'            AND text            IS NOT NULL) OR
        (kind = 'link'            AND url             IS NOT NULL)
    )
);

INSERT INTO canvas_nodes_new (
    id, topic_id, kind, x, y, width, height, z,
    note_id, target_topic_id, asset_id, question_id, ink, text, url, color,
    frame_id, order_in_frame, created_at, updated_at
)
SELECT id, topic_id, kind, x, y, width, height, z,
       note_id, target_topic_id, asset_id, NULL, ink, text, url, color,
       frame_id, order_in_frame, created_at, updated_at
  FROM canvas_nodes;

DROP TABLE canvas_nodes;
ALTER TABLE canvas_nodes_new RENAME TO canvas_nodes;

CREATE INDEX idx_canvas_nodes_topic    ON canvas_nodes (topic_id);
CREATE INDEX idx_canvas_nodes_note     ON canvas_nodes (note_id);
CREATE INDEX idx_canvas_nodes_frame    ON canvas_nodes (frame_id, order_in_frame);
CREATE INDEX idx_canvas_nodes_asset    ON canvas_nodes (asset_id);
CREATE INDEX idx_canvas_nodes_question ON canvas_nodes (question_id);
