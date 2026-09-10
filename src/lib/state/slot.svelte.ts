/**
 * The right-hand slot: an inspector for whatever the centre is showing.
 *
 * It is never a destination. Library, Journal, Tags and Tasks are screens you
 * navigate to in the middle of the window; what survives here is only what
 * describes something already on screen — this note's info, this board's
 * questions, this day's tasks — plus the card list on a board, whose whole job
 * is to be a drag source for the canvas beside it.
 *
 * The tool row always holds the same six icons in the same order, and all six
 * are always live. A panel that rearranges itself as you navigate is one you
 * can never learn, and a tool you cannot press is a tool you cannot find out
 * about: pressing Info with no note open says so in a sentence, which teaches
 * you what Info is for.
 *
 * Closed means **gone**, not a strip of icons down the edge. A rail is a panel
 * that still costs you width while giving you nothing; the space belongs to the
 * centre. It is reopened from the top bar.
 *
 * **The lock is the important part of this file.**
 *
 * The guarantee memory mode needs is narrow: while you are writing blind, the
 * source must be unreachable. It has to stay *structural* — the pane not in the
 * DOM — because a policy with a button next to it is not a guarantee.
 *
 * It used to shut the whole panel to achieve that, which was far too much. The
 * library, the day's tasks and the outline have nothing to do with the source,
 * and losing them mid-recall made the app look broken. So the lock now removes
 * exactly one tool: Source leaves the row, and the active tool is pushed off it
 * if that is where you were. Everything else keeps working.
 *
 * If that narrowing is ever widened back into "disable the button", the rule the
 * app exists for has a back door.
 */

import type { Frame, Topic } from '$lib/types';
import type { Heading } from '$lib/outline';

/** What the centre is showing. Decides which tools exist at all. */
export type SlotContext = 'canvas' | 'note' | 'journal' | 'none';

export type SlotTool = 'library' | 'info' | 'source' | 'questions' | 'tasks' | 'outline';

export const SLOT_LABEL: Record<SlotTool, string> = {
	library: 'Cards',
	info: 'Info',
	source: 'Source',
	questions: 'Questions',
	tasks: 'Tasks',
	outline: 'Outline'
};

const OPEN_KEY = 'study.slotOpen';
const TOOL_KEY = 'study.slotTool';

class Slot {
	open = $state(true);
	tool = $state<SlotTool>('library');

	/** Set while a blind write is in progress. Source leaves the row entirely. */
	locked = $state(false);

	/** The note the Info and Source tools describe, when one is open. */
	noteId = $state<string | null>(null);
	/** The board in view, for Questions and Outline. */
	topic = $state<Topic | null>(null);
	/** The frames on that board, for Outline. */
	frames = $state<Frame[]>([]);
	/** What the Source tool renders: the long note being distilled from. */
	sourceNoteId = $state<string | null>(null);
	/** The day the journal is showing, so Tasks describes the same date. */
	day = $state<string | null>(null);
	/** Set by the editor, so the Source tool can offer to change the source. */
	requestSourcePick = $state<(() => void) | undefined>(undefined);
	/** Set by whatever screen can open a note, so a backlink is followable. */
	openNote = $state<((noteId: string) => void) | undefined>(undefined);
	/** Set by the canvas so the Outline can fly to a frame. */
	focusFrame = $state<((frameId: string) => void) | undefined>(undefined);
	/** The headings of whatever is being written in the centre, for Outline. */
	headings = $state<Heading[]>([]);
	/** Set by whichever editor owns those headings, so one can be jumped to. */
	focusHeading = $state<((pos: number) => void) | undefined>(undefined);

	/**
	 * Called by an editor whose document changed shape.
	 *
	 * Whoever is writing owns the outline, which is why this replaces rather
	 * than merges: a note opened over the day's page is the thing in front of
	 * you, and its headings are the ones worth listing. When it closes, the page
	 * underneath publishes again.
	 */
	setOutline(headings: Heading[], focus: (pos: number) => void) {
		this.headings = headings;
		this.focusHeading = focus;
	}

	/**
	 * Notes an editor currently has open, and how to tick a task inside one.
	 *
	 * While an editor holds a document, that editor's copy is the live one —
	 * anything that writes the note's body behind it will be overwritten the
	 * next time it saves. So an edit that could be made through the editor is
	 * made through the editor, and the database is only written directly when
	 * nothing has the note open.
	 *
	 * Not reactive: it is read at the moment of a click, never rendered.
	 */
	#open = new Map<string, (index: number, done: boolean) => boolean>();

	/** Called by an editor for as long as it holds a note. Returns its release. */
	holdDocument(noteId: string, setTask: (index: number, done: boolean) => boolean) {
		this.#open.set(noteId, setTask);
		return () => {
			// Only if it is still ours: a note opened over the page it belongs to
			// replaces this entry, and its release must not take the new one away.
			if (this.#open.get(noteId) === setTask) this.#open.delete(noteId);
		};
	}

	/**
	 * Ticks a task through whichever editor holds its note.
	 *
	 * False means nobody does, and the caller should write to the database as
	 * usual. A task id is `<note id>:<position>`.
	 */
	setTaskInOpenDocument(taskId: string, done: boolean): boolean {
		const at = taskId.lastIndexOf(':');
		if (at < 0) return false;

		const setTask = this.#open.get(taskId.slice(0, at));
		return setTask ? setTask(Number(taskId.slice(at + 1)), done) : false;
	}

	/** Called when a board opens, so every tool describes the same thing. */
	enterBoard(topic: Topic, frames: Frame[], focus: (frameId: string) => void) {
		this.topic = topic;
		this.frames = frames;
		this.focusFrame = focus;
	}

	/** Called when an editor opens, so Info and Source describe that note. */
	enterNote(noteId: string, sourceNoteId: string | null, pickSource?: () => void) {
		this.noteId = noteId;
		this.sourceNoteId = sourceNoteId;
		this.requestSourcePick = pickSource;
	}

	leaveNote() {
		this.noteId = null;
		this.sourceNoteId = null;
		this.requestSourcePick = undefined;
	}

	constructor() {
		try {
			this.open = localStorage.getItem(OPEN_KEY) !== '0';
			const stored = localStorage.getItem(TOOL_KEY);
			if (stored && stored in SLOT_LABEL) this.tool = stored as SlotTool;
		} catch {
			/* defaults are fine */
		}
	}

	/**
	 * Every tool, in a fixed order, everywhere.
	 *
	 * The row does not change shape as you navigate. Hiding the tools that do
	 * not apply meant the panel rearranged itself under you — icons moved, the
	 * whole thing vanished on some screens — and you could never learn where
	 * anything was. Showing all six and dimming the ones that cannot speak is
	 * the honest version: it says *this exists, not here*.
	 */
	private readonly allTools: SlotTool[] = [
		'library',
		'info',
		'source',
		'questions',
		'tasks',
		'outline'
	];

	/** Source is absent, not disabled, while a blind write is in progress. */
	get tools(): SlotTool[] {
		return this.locked ? this.allTools.filter((t) => t !== 'source') : this.allTools;
	}

	/**
	 * Whether a tool can describe what the centre is showing.
	 *
	 * Never used to disable anything — every tool stays pressable. It decides
	 * whether the panel renders the tool or the one sentence saying what the
	 * tool would need in order to say something.
	 */
	applies(tool: SlotTool, context: SlotContext): boolean {
		switch (tool) {
			// The corpus and the day exist regardless of what is on screen.
			case 'library':
			case 'tasks':
				return true;
			case 'info':
			case 'source':
				return context === 'note';
			// Doubts exist wherever you are. Requiring a board meant a question
			// caught in the journal was invisible at exactly the moment you
			// wanted it; the panel scopes instead of hiding.
			case 'questions':
				return true;
			// The day's page is a document like any other, and it is the one you
			// write most. Leaving it out meant the panel promised headings and
			// then had nothing to say on the only screen you were using.
			case 'outline':
				return context === 'canvas' || context === 'note' || context === 'journal';
		}
	}

	/** What the tool would need. A plain fact, never a reproach. */
	needs(tool: SlotTool): string {
		switch (tool) {
			case 'info':
				return 'Open a note and this shows where it lives, what points at it, and how it stands with memory.';
			case 'source':
				return 'Open a note distilled from a longer one and this shows what it came from.';

			case 'outline':
				return 'Open a board and this lists its sections; write in a note or a day and it lists the headings.';
			default:
				return '';
		}
	}

	show(tool: SlotTool) {
		// The one thing the lock forbids. Everything else stays reachable.
		if (this.locked && tool === 'source') return;
		this.tool = tool;
		this.open = true;
		this.persist();
	}

	toggle() {
		this.open = !this.open;
		this.persist();
	}

	/**
	 * Called when a blind write begins.
	 *
	 * Takes Source out of the row and moves you off it if that is where you
	 * were. The panel itself stays: nothing else in it can show you the source,
	 * and closing the whole thing mid-recall was a far bigger loss than the
	 * guarantee needed.
	 */
	lock() {
		this.locked = true;
		if (this.tool === 'source') this.tool = 'info';
	}

	unlock() {
		this.locked = false;
	}

	private persist() {
		try {
			localStorage.setItem(OPEN_KEY, this.open ? '1' : '0');
			localStorage.setItem(TOOL_KEY, this.tool);
		} catch {
			/* it just will not persist */
		}
	}
}

export const slot = new Slot();
