-- Splits recall and accuracy into two independent axes.
--
-- Axis 1 is the recall state, derived from the facts below.
-- Axis 2 is a separate marker: recalled but never contrasted against a source,
-- which is the "confidently wrong" case. It has to be able to coexist with
-- `recalled`, which is exactly why it cannot be one of the states.

-- Written by the Contrast step: the moment the source is revealed and I say
-- whether what I wrote matches it.
ALTER TABLE notes ADD COLUMN last_checked_at TEXT;

-- Replaces a boolean. Keeping *when* a recall failed lets "recalled but
-- failing" and "never recalled" stay distinguishable, and a later successful
-- recall supersedes it by being more recent -- so nothing is ever erased.
ALTER TABLE notes ADD COLUMN last_recall_failed_at TEXT;

-- When the body was last changed by ordinary editing. A blind rewrite in
-- memory mode deliberately does NOT touch this, otherwise every recall would
-- poison the next one. This is what the cooling rule reads.
ALTER TABLE notes ADD COLUMN content_edited_at TEXT;

ALTER TABLE notes RENAME COLUMN hash_at_recall TO content_hash_at_recall;

-- `last_recall_closed_book` stored the same fact twice: memory mode is the only
-- writer of a recall, so a recall is closed-book by construction. Storing the
-- flag alongside was the `fromMemory` mistake in a second costume.
ALTER TABLE notes DROP COLUMN last_recall_closed_book;

-- Superseded by last_recall_failed_at, which keeps the timestamp.
ALTER TABLE notes DROP COLUMN last_recall_failed;

-- Backfill. Existing notes with a body were written at some point, so they are
-- distilled and have been edited; without this they would all read as `raw`.
UPDATE notes
   SET content_edited_at = updated_at,
       distilled_at = COALESCE(distilled_at, created_at)
 WHERE trim(body_text) <> '';

-- A card is a projection of an atomic note -- front is the title, back is the
-- body -- so there is nothing to author and nothing to store. Only scheduling
-- state will be kept, and that arrives with the review queue. Storing front and
-- back here would be storing a copy that can drift from the note it came from.
DROP TABLE IF EXISTS cards;
