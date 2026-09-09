-- Rich text needs three projections of the same body, not one:
--
--   body_json  the TipTap document, so the editor can reload it losslessly
--   body_html  the rendered result, so the assembled document can show real
--              formatting instead of flattened plain text
--   body_text  the plaintext projection, which is what search reads and what
--              the content hash is taken over -- so restyling a note never
--              costs you a verification
ALTER TABLE notes ADD COLUMN body_html TEXT NOT NULL DEFAULT '';
