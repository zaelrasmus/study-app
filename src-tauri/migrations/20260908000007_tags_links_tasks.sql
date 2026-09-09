-- Tags, mentions and tasks.
--
-- Two of these three are *derived indexes*, not entities: `note_links` and
-- `note_tasks` are rebuilt from the note body on every save and are never a
-- source of truth. They exist because a body is opaque to SQL — you cannot ask
-- "what links here" or "what is due today" of a JSON blob — and for no other
-- reason. Delete either table and a re-save reconstructs it.

-- Tags are a real entity, and the only genuinely new one here.
CREATE TABLE tags (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    -- Spanish-folded, so `Localización` and `localizacion` are one tag while
    -- `año` and `ano` stay two. Uniqueness lives on the fold, not the name.
    fold       TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE note_tags (
    note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    tag_id  TEXT NOT NULL REFERENCES tags  (id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);

CREATE INDEX idx_note_tags_tag ON note_tags (tag_id);

-- Mentions. Deliberate by construction: a mention only exists because you
-- picked a note from a dialog, so there is nothing here to suggest, nothing to
-- auto-link, and no "you might want to link this". Linking everywhere buries
-- the connections that actually matter.
CREATE TABLE note_links (
    from_note_id TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    to_note_id   TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    PRIMARY KEY (from_note_id, to_note_id)
);

CREATE INDEX idx_note_links_to ON note_links (to_note_id);

-- Tasks are an inline block in any note, not a separate thing you file. This
-- table only makes them queryable.
--
-- `due_on` is null unless the text carries an explicit `due:YYYY-MM-DD`. That
-- is the whole point: a task with no date never surfaces in the Tasks view, so
-- the view can only ever show a day's worth. There is no total of everything
-- open, and no overdue count.
CREATE TABLE note_tasks (
    id       TEXT PRIMARY KEY,
    note_id  TEXT NOT NULL REFERENCES notes (id) ON DELETE CASCADE,
    text     TEXT    NOT NULL,
    done     INTEGER NOT NULL DEFAULT 0,
    due_on   TEXT,
    position INTEGER NOT NULL
);

CREATE INDEX idx_note_tasks_due  ON note_tasks (due_on);
CREATE INDEX idx_note_tasks_note ON note_tasks (note_id);
