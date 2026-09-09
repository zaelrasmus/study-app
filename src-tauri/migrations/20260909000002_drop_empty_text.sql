-- Empty loose text is never a thought you had.
--
-- The text tool briefly wrote its node to the database before you had typed
-- anything, so abandoning the click left a node with no text — invisible on the
-- board, since loose text draws no border until you touch it. The tool now
-- holds the unwritten node in memory and only saves it once there are words,
-- so nothing new can land here; this clears what the earlier version left.
DELETE FROM canvas_nodes
 WHERE kind = 'text'
   AND (text IS NULL OR trim(text) = '');
