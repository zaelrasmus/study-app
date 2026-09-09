/**
 * Putting a date on a task.
 *
 * The date has always been `due:YYYY-MM-DD`, written inline — explicit, because
 * a date the app guessed out of your prose is worse than no date at all. But
 * typing fourteen characters and knowing today's date is a wall, not a
 * mechanism, and the result was that no task ever got dated.
 *
 * So the *cost* moves, not the rule. You pick "today" from the slash menu and
 * the real ISO date is written into the text for you. What is stored is still
 * an explicit date that you chose; the app never inferred it from anything.
 *
 * These only appear while the cursor is inside a task. A due date on a
 * paragraph would index nothing, so offering it there would be a lie.
 *
 * Reaching them means typing a space and then `/`: TipTap's suggestion matcher
 * only fires on a `/` that begins a block or follows whitespace, which is not
 * something this file can change without replacing the matcher.
 */

import CalendarIcon from '@lucide/svelte/icons/calendar';
import CalendarCheckIcon from '@lucide/svelte/icons/calendar-check';
import CalendarClockIcon from '@lucide/svelte/icons/calendar-clock';
// The core editor type, not Edra's: these only read the selection and run a
// chain, so the narrower type is both accurate and what the extension hands us.
import type { Editor } from '@tiptap/core';
import type { EdraCommand } from './commands.ts';

function iso(d: Date): string {
	const month = String(d.getMonth() + 1).padStart(2, '0');
	const day = String(d.getDate()).padStart(2, '0');
	return `${d.getFullYear()}-${month}-${day}`;
}

function addDays(from: Date, days: number): Date {
	const d = new Date(from);
	d.setDate(d.getDate() + days);
	return d;
}

/**
 * Replaces any date already on the task, rather than appending a second one.
 *
 * Two `due:` markers in one line would be a task with two answers to "when",
 * and the index reads the first — so the text and the behaviour would disagree.
 *
 * Positions are found by scanning the item's own text nodes rather than by
 * arithmetic on `textContent`: a task can hold marks, links or several text
 * nodes, and offsets into the concatenated string do not survive any of that.
 */
function setDue(editor: Editor, date: string) {
	const { state } = editor;
	const { $from } = state.selection;

	for (let depth = $from.depth; depth > 0; depth--) {
		if ($from.node(depth).type.name !== 'taskItem') continue;

		const item = $from.node(depth);
		const start = $from.start(depth);
		const end = start + item.content.size;

		let found: { from: number; to: number } | null = null;
		state.doc.nodesBetween(start, end, (node, pos) => {
			if (found || !node.isText || !node.text) return;
			const at = node.text.indexOf('due:');
			if (at === -1) return;

			// Only the marker and the ten characters of a date follow it.
			const tail = node.text.slice(at, at + 14);
			found = { from: pos + at, to: pos + at + tail.length };
		});

		if (found) {
			editor.chain().focus().insertContentAt(found, `due:${date}`).run();
		} else {
			// Append inside the paragraph the cursor is in, so the date joins the
			// sentence rather than becoming a block of its own.
			//
			// TipTap's suggestion only fires on a `/` at the start of a block or
			// after a space, so reaching this menu mid-sentence leaves that space
			// behind once the "/" is deleted. Adding another would give every
			// dated task a double gap.
			const paragraph = $from.parent.textContent;
			const gap = paragraph.length === 0 || /\s$/.test(paragraph) ? '' : ' ';
			editor.chain().focus().insertContentAt($from.end(), `${gap}due:${date}`).run();
		}
		return;
	}
}

/** The three that cover nearly every case, plus a week out. */
export const dueCommands: EdraCommand[] = [
	{
		icon: CalendarCheckIcon,
		name: 'dueToday',
		tooltip: 'Due today',
		onClick: (editor: Editor) => setDue(editor, iso(new Date()))
	},
	{
		icon: CalendarClockIcon,
		name: 'dueTomorrow',
		tooltip: 'Due tomorrow',
		onClick: (editor: Editor) => setDue(editor, iso(addDays(new Date(), 1)))
	},
	{
		icon: CalendarIcon,
		name: 'dueNextWeek',
		tooltip: 'Due in a week',
		onClick: (editor: Editor) => setDue(editor, iso(addDays(new Date(), 7)))
	}
];

/** Whether a due date would mean anything where the cursor is. */
export function inTask(editor: Editor): boolean {
	const { $from } = editor.state.selection;
	for (let depth = $from.depth; depth > 0; depth--) {
		if ($from.node(depth).type.name === 'taskItem') return true;
	}
	return false;
}
