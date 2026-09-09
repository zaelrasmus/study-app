-- Things that are not notes: files you brought in, and pages you point at.
--
-- A board is where thinking happens, and thinking happens against sources — a
-- diagram, a paper, a page you read. Until now the only thing you could put on
-- a canvas was your own writing, which meant the source stayed in another
-- application and the board was only ever half the picture.
--
-- None of these are notes. They carry no title, no knowledge state and no card,
-- and they never enter review: they are what you think *about*, not what you
-- have learned. Distilling one into a note is a separate, deliberate act.

-- Where the file was put, relative to the app's own asset directory. Stored
-- relative so moving or backing up the data directory does not break every
-- image at once.
ALTER TABLE assets ADD COLUMN rel_path TEXT NOT NULL DEFAULT '';

-- SQLite cannot alter a CHECK, so the table is rebuilt to admit 'link'.
--
-- A link is not an asset: there is no file, nothing was copied, and the page can
-- change under you. It lives on the node itself.
CREATE TABLE canvas_nodes_new (
    id       TEXT PRIMARY KEY,
    topic_id TEXT NOT NULL REFERENCES topics (id) ON DELETE CASCADE,
    kind     TEXT NOT NULL
        CHECK (kind IN ('note', 'ink', 'image', 'pdf', 'topic', 'text', 'link')),

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
        (kind = 'ink'             AND ink             IS NOT NULL) OR
        (kind = 'text'            AND text            IS NOT NULL) OR
        (kind = 'link'            AND url             IS NOT NULL)
    )
);

INSERT INTO canvas_nodes_new (
    id, topic_id, kind, x, y, width, height, z,
    note_id, target_topic_id, asset_id, ink, text, url, color,
    frame_id, order_in_frame, created_at, updated_at
)
SELECT id, topic_id, kind, x, y, width, height, z,
       note_id, target_topic_id, asset_id, ink, text, NULL, color,
       frame_id, order_in_frame, created_at, updated_at
  FROM canvas_nodes;

DROP TABLE canvas_nodes;
ALTER TABLE canvas_nodes_new RENAME TO canvas_nodes;

CREATE INDEX idx_canvas_nodes_topic ON canvas_nodes (topic_id);
CREATE INDEX idx_canvas_nodes_note  ON canvas_nodes (note_id);
CREATE INDEX idx_canvas_nodes_frame ON canvas_nodes (frame_id, order_in_frame);
CREATE INDEX idx_canvas_nodes_asset ON canvas_nodes (asset_id);
