/**
 * Ticking a checkbox that lives in a document someone is editing.
 *
 * `set_task_done` writes the note's body straight into the database. That is
 * correct when nothing has the note open, and quietly destructive when
 * something does: the editor still holds the document as it was a moment ago,
 * and the next time it saves — a keystroke, changing day, walking away from the
 * screen — it writes its own copy over the top and the tick is gone.
 *
 * That is what made a task ticked in the inspector come back unticked a few
 * seconds later, while the same tick made in the note itself stuck.
 *
 * So when an editor holds the note, the tick is made *there* instead. It is the
 * same edit, made in the one place that owns the document, and it has the side
 * benefit of being visible: the checkbox in the note beside the panel moves
 * when you click the panel.
 */

import type { Editor } from '@tiptap/core';

/**
 * Ticks the nth task in a document, counting the way the backend counts.
 *
 * The index is the position a task id was derived from, and the backend skips
 * task items whose text is empty — so this skips them too. Counting them would
 * drift the moment you left a checkbox blank, and the tick would land on the
 * wrong line.
 *
 * Returns false when there is no such task, which is the signal to fall back to
 * writing through the database.
 */
export function setTaskChecked(
	editor: Editor | undefined | null,
	index: number,
	done: boolean
): boolean {
	if (!editor || !Number.isInteger(index) || index < 0) return false;

	let seen = 0;
	let target: number | null = null;
	let attrs: Record<string, unknown> = {};

	editor.state.doc.descendants((node, pos) => {
		if (target !== null) return false;
		if (node.type.name !== 'taskItem') return true;

		// Never descend into a task item: the backend does not either, so a
		// nested list would be counted here and not there.
		if (node.textContent.trim() === '') return false;
		if (seen === index) {
			target = pos;
			attrs = { ...node.attrs };
		}
		seen += 1;
		return false;
	});

	if (target === null) return false;
	if (attrs.checked === done) return true;

	const at = target;
	return editor
		.chain()
		.command(({ tr }) => {
			tr.setNodeMarkup(at, undefined, { ...attrs, checked: done });
			return true;
		})
		.run();
}
