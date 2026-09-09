-- Three additions, all of them things the canvas and the journal could not say.
--
-- 1. A day can have a note of its own, so the journal is somewhere you write
--    rather than only a list of what you caught.
-- 2. The canvas can hold loose text that is not a note yet.
-- 3. Nodes can point at each other.

-- 1 -------------------------------------------------------------------------
--
-- The journal stays a filter over captures: the day's captures are still just
-- notes with a creation date, and nothing here groups them. What this adds is
-- one *further* note per day — the page you write on — which is an ordinary
-- note in every other respect: it can be distilled, recalled, tagged, placed on
-- a board. The column only records which day's page it is.
ALTER TABLE notes ADD COLUMN journal_day TEXT;

-- One page per day. Partial, so the millions of notes that are not day pages
-- are not forced to be distinct on NULL.
CREATE UNIQUE INDEX idx_notes_journal_day
    ON notes (journal_day) WHERE journal_day IS NOT NULL;

-- 2 -------------------------------------------------------------------------
--
-- SQLite cannot alter a CHECK, so the table is rebuilt to admit 'text'.
-- Loose text is the cheapest possible thing to put on a canvas: no title, no
-- state, no card. It exists so that thinking out loud next to your cards does
-- not first require deciding that the thought is a note.
CREATE TABLE canvas_nodes_new (
    id       TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,
    kind     TEXT NOT NULL CHECK (kind IN ('note', 'ink', 'image', 'pdf', 'topic', 'text')),

    x      REAL NOT NULL DEFAULT 0,
    y      REAL NOT NULL DEFAULT 0,
    width  REAL,
    height REAL,
    z      INTEGER NOT NULL DEFAULT 0,

    note_id         TEXT REFERENCES notes  (id) ON DELETE CASCADE,
    target_topic_id TEXT REFERENCES topics (id) ON DELETE CASCADE,
    asset_id        TEXT REFERENCES assets (id) ON DELETE CASCADE,
    ink             TEXT,
    text            TEXT,

    color TEXT,

    frame_id       TEXT REFERENCES frames (id) ON DELETE SET NULL,
    order_in_frame INTEGER,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (
        (kind = 'note'            AND note_id         IS NOT NULL) OR
        (kind = 'topic'           AND target_topic_id IS NOT NULL) OR
        (kind IN ('image', 'pdf') AND asset_id        IS NOT NULL) OR
        (kind = 'ink'             AND ink             IS NOT NULL) OR
        (kind = 'text'            AND text            IS NOT NULL)
    )
);

INSERT INTO canvas_nodes_new (
    id, topic_id, kind, x, y, width, height, z,
    note_id, target_topic_id, asset_id, ink, text, color,
    frame_id, order_in_frame, created_at, updated_at
)
SELECT id, topic_id, kind, x, y, width, height, z,
       note_id, target_topic_id, asset_id, ink, NULL, color,
       frame_id, order_in_frame, created_at, updated_at
  FROM canvas_nodes;

DROP TABLE canvas_nodes;
ALTER TABLE canvas_nodes_new RENAME TO canvas_nodes;

CREATE INDEX idx_canvas_nodes_topic ON canvas_nodes (topic_id);
CREATE INDEX idx_canvas_nodes_note  ON canvas_nodes (note_id);
CREATE INDEX idx_canvas_nodes_frame ON canvas_nodes (frame_id, order_in_frame);

-- 3 -------------------------------------------------------------------------
--
-- A pointer from one node to another, drawn on the board.
--
-- Deliberately *not* a link between notes. Note-to-note links are derived from
-- the body text and rebuilt on every save, so they can never drift from what
-- you wrote. A pointer is the other thing: a spatial claim that these two
-- cards, arranged this way, on this board, belong together. It lives with the
-- arrangement, so it dies with the arrangement.
CREATE TABLE canvas_edges (
    id       TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,

    source_id TEXT NOT NULL REFERENCES canvas_nodes (id) ON DELETE CASCADE,
    target_id TEXT NOT NULL REFERENCES canvas_nodes (id) ON DELETE CASCADE,

    -- Optional, and usually empty. A label on every arrow is a diagram; the
    -- point here is the arrangement, not the annotation.
    label TEXT NOT NULL DEFAULT '',

    created_at TEXT NOT NULL,

    CHECK (source_id <> target_id)
);

CREATE INDEX idx_canvas_edges_topic ON canvas_edges (topic_id);

-- One arrow per ordered pair. Drawing the same pointer twice is not two facts.
CREATE UNIQUE INDEX idx_canvas_edges_pair ON canvas_edges (source_id, target_id);
