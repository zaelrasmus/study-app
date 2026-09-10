/**
 * Room to write at the foot of a document.
 *
 * Two separate things, and both are needed.
 *
 * **The run-off** is padding at the end of the writing surface, applied where
 * the editor is rendered. It is what makes it *possible* to scroll past the
 * last line at all — without it the document simply ends, and the bottom of the
 * page is the bottom of the window.
 *
 * **The floor** is this file. Padding alone does not help while you are
 * actually typing: the browser scrolls just far enough to reveal the caret, so
 * the line you are writing sits flat against the bottom edge no matter how much
 * empty space is underneath it. That is the part that feels wrong, and it is
 * worst exactly where it matters most — the messy end of a day's thinking,
 * where you are adding lines rather than editing old ones.
 *
 * ProseMirror already knows how to do this. `scrollThreshold` says how close to
 * the edge the caret may drift before the view scrolls, and `scrollMargin` says
 * how much clear space to leave once it does. Setting both to the same figure
 * gives a soft floor: the caret rises off the bottom edge and stays there while
 * you write.
 */

import type { Editor } from '@tiptap/core';

/**
 * How much clear space is kept under the caret.
 *
 * Roughly six lines. Enough that the sentence you are writing has somewhere to
 * go, and not so much that a short page jumps around under you.
 */
const FLOOR = 220;

/** A little at the top too, so the line above is never half under the bar. */
const CEILING = 60;

/**
 * Keeps clear space under the caret for the life of this editor.
 *
 * Safe to call before the view is mounted: the options are read again when the
 * view is built, so it applies either way.
 */
export function keepRoomBelow(editor: Editor | undefined | null) {
	editor?.setOptions({
		editorProps: {
			scrollThreshold: { top: CEILING, right: 0, bottom: FLOOR, left: 0 },
			scrollMargin: { top: CEILING, right: 0, bottom: FLOOR, left: 0 }
		}
	});
}
