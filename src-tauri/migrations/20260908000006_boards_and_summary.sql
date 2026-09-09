-- Work boards and study boards, and the card summary.

-- One boolean, and it governs *presentation only*.
--
-- It never gates reviewability. A card exists because you reproduced something
-- from memory, not because of which surface the note happens to sit on -- and
-- since a note can live on a work board and a study board at the same time,
-- any other rule would mean removing it from one board silently removed it
-- from review. That is the same failure as suspending a drifted card: the
-- system switching itself off without telling you.
--
-- Work board is the default, including for topics that already exist. Study is
-- the mode you opt into for something you are actually trying to learn, not the
-- mode you opt out of every time you want to think.
ALTER TABLE topics ADD COLUMN study INTEGER NOT NULL DEFAULT 0;

-- One line describing the concept.
--
-- Heptabase's rule is that a full sentence should be the title, so that months
-- later the title alone recovers the concept. That cannot be the title here,
-- because the title is the review front and a sentence answers its own card.
-- So it is a separate field: the summary is what the canvas, the library and
-- link previews render; the title is what review asks.
ALTER TABLE notes ADD COLUMN summary TEXT NOT NULL DEFAULT '';
