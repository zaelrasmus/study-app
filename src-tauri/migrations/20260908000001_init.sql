-- Core schema.
--
-- Two ideas drive the shape of this file:
--
--   1. Note status is NOT stored. `raw / draft / verified / stale` are derived
--      in Rust from the provenance columns on `notes` plus the current content
--      hash, so there are no illegal transitions and no stuck states.
--
--   2. Placement is membership. A note belongs to a topic because it sits on
--      that topic's canvas -- there is no join table. Notes with no canvas node
--      anywhere are the capture inbox.

-- Topics are workspaces, not folders: no path, no parent, no tree. Topics nest
-- spatially instead, by appearing as a node on another topic's canvas.
CREATE TABLE topics (
    id              TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    -- Drives the sidebar. Topics not visited recently fall asleep on their own.
    last_visited_at TEXT NOT NULL
);

CREATE INDEX idx_topics_last_visited ON topics (last_visited_at DESC);

-- `kind` is a stage, not a type: long and atomic are the same entity at
-- different points in its life.
CREATE TABLE notes (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL DEFAULT 'long' CHECK (kind IN ('long', 'atomic')),
    title        TEXT NOT NULL DEFAULT '',
    -- TipTap document, verbatim, for the editor to reload.
    body_json    TEXT NOT NULL DEFAULT '',
    -- Plaintext projection: what search reads and what the hash is taken over,
    -- so that reformatting alone never invalidates a recall.
    body_text    TEXT NOT NULL DEFAULT '',
    content_hash TEXT NOT NULL DEFAULT '',

    -- Provenance facts. Status is a function of these; see domain::note.
    distilled_at            TEXT,
    last_recall_at          TEXT,
    last_recall_closed_book INTEGER NOT NULL DEFAULT 0,
    last_recall_failed      INTEGER NOT NULL DEFAULT 0,
    -- Content hash captured at the moment of the last recall. If it no longer
    -- matches content_hash, the note has drifted from what was recalled.
    hash_at_recall          TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_notes_updated ON notes (updated_at DESC);

CREATE TABLE assets (
    id         TEXT PRIMARY KEY,
    kind       TEXT    NOT NULL CHECK (kind IN ('image', 'pdf')),
    file_name  TEXT    NOT NULL,
    mime       TEXT    NOT NULL,
    byte_size  INTEGER NOT NULL,
    created_at TEXT    NOT NULL
);

-- A labelled region of a topic's canvas. Frames are the unit of document
-- assembly and the unit of completion: you finish a frame, never a topic.
--
-- order_index is explicit and rendered as a visible badge on the frame. It is
-- never inferred from x/y, so rearranging the canvas to think better cannot
-- silently rewrite the document.
CREATE TABLE frames (
    id          TEXT PRIMARY KEY,
    topic_id    TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,
    label       TEXT NOT NULL DEFAULT '',
    x           REAL NOT NULL DEFAULT 0,
    y           REAL NOT NULL DEFAULT 0,
    width       REAL NOT NULL DEFAULT 640,
    height      REAL NOT NULL DEFAULT 420,
    order_index INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_frames_topic ON frames (topic_id, order_index);

-- Everything placed on a canvas. `frame_id` is the commitment gesture: a node
-- inside a frame enters the assembled document, a node outside every frame
-- stays scratch. The canvas is always a superset of the document.
CREATE TABLE canvas_nodes (
    id       TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,
    kind     TEXT NOT NULL CHECK (kind IN ('note', 'ink', 'image', 'pdf', 'topic')),

    x      REAL NOT NULL DEFAULT 0,
    y      REAL NOT NULL DEFAULT 0,
    width  REAL,
    height REAL,
    z      INTEGER NOT NULL DEFAULT 0,

    -- Exactly one of these carries the payload, enforced by the CHECK below.
    note_id         TEXT REFERENCES notes  (id) ON DELETE CASCADE,
    target_topic_id TEXT REFERENCES topics (id) ON DELETE CASCADE,
    asset_id        TEXT REFERENCES assets (id) ON DELETE CASCADE,
    ink             TEXT,

    frame_id       TEXT REFERENCES frames (id) ON DELETE SET NULL,
    order_in_frame INTEGER,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (
        (kind = 'note'            AND note_id         IS NOT NULL) OR
        (kind = 'topic'           AND target_topic_id IS NOT NULL) OR
        (kind IN ('image', 'pdf') AND asset_id        IS NOT NULL) OR
        (kind = 'ink'             AND ink             IS NOT NULL)
    )
);

CREATE INDEX idx_canvas_nodes_topic ON canvas_nodes (topic_id);
CREATE INDEX idx_canvas_nodes_note  ON canvas_nodes (note_id);
CREATE INDEX idx_canvas_nodes_frame ON canvas_nodes (frame_id, order_in_frame);

-- Questions are first-class, never text inside a note.
--
-- Priority comes from origin, never from date: a gap found while reconstructing
-- from memory outranks a review failure, which outranks something jotted down
-- while reading. Date is used only to break ties inside one origin bucket.
CREATE TABLE questions (
    id     TEXT NOT NULL PRIMARY KEY,
    text   TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open'
           CHECK (status IN ('open', 'investigating', 'resolved', 'abandoned', 'dissolved')),
    origin TEXT NOT NULL
           CHECK (origin IN ('recall_gap', 'review_failure', 'reading')),

    topic_id       TEXT REFERENCES topics (id) ON DELETE SET NULL,
    -- Where the question came from.
    source_note_id TEXT REFERENCES notes  (id) ON DELETE SET NULL,
    -- Required to reach 'resolved': closing a question means linking the note
    -- that answers it.
    answer_note_id TEXT REFERENCES notes  (id) ON DELETE SET NULL,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (status <> 'resolved' OR answer_note_id IS NOT NULL)
);

CREATE INDEX idx_questions_status ON questions (status, origin);
CREATE INDEX idx_questions_topic  ON questions (topic_id);

-- Cards are derived from atomic notes and die with them. A card whose note is
-- no longer verified suspends rather than drilling possibly-wrong material.
CREATE TABLE cards (
    id      TEXT PRIMARY KEY,
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    front   TEXT NOT NULL,
    back    TEXT NOT NULL,

    due_at        TEXT NOT NULL,
    interval_days REAL    NOT NULL DEFAULT 0,
    ease          REAL    NOT NULL DEFAULT 2.5,
    reps          INTEGER NOT NULL DEFAULT 0,
    lapses        INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_cards_due  ON cards (due_at);
CREATE INDEX idx_cards_note ON cards (note_id);
