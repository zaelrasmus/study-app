/**
 * Per-machine workspace preferences.
 *
 * These are ergonomics, not domain state: where a panel sits, not what is true
 * about a note. That is why they live in `localStorage` rather than the
 * database, and why there are so few of them — a settings surface is a fiddling
 * surface, and this app has exactly one thing worth fiddling with.
 */

/**
 * How the note editor is presented.
 *
 * `full` fills the working area between the sidebar and the window edge, which
 * is the only presentation with enough room for the memory-mode split view.
 * `dialog` is a large centred modal for when the canvas underneath is worth
 * keeping in peripheral vision.
 */
export type EditorPresentation = 'full' | 'dialog';

/**
 * Where the app opens.
 *
 * The day's page, by default. The question queue was the old landing and it is
 * the wrong greeting: a list of things you have not answered is a bill, and the
 * one thing this app has to protect is the moment you sit down with something
 * to write down. Capture first; the queue is a place you choose to go.
 *
 * Stored rather than hard-coded because the right answer is personal, and the
 * queue kept its own address when this moved, so switching costs nothing.
 */
export type StartPage = 'journal' | 'questions';

const START_KEY = 'study.startPage';
const KEY = 'study.editorPresentation';
const LEFT_KEY = 'study.sidebarWidth';
const RIGHT_KEY = 'study.slotWidth';

/** Both panels clamp, so neither can be dragged into uselessness. */
export const SIDEBAR_MIN = 180;
export const SIDEBAR_MAX = 380;
export const SLOT_MIN = 240;
export const SLOT_MAX = 560;

class Prefs {
	editorPresentation = $state<EditorPresentation>('full');
	startPage = $state<StartPage>('journal');
	sidebarWidth = $state(216);
	slotWidth = $state(300);

	/** Where `/` sends you. The only place that turns the choice into a route. */
	get startPath() {
		return this.startPage === 'questions' ? '/questions' : '/journal';
	}

	constructor() {
		// Storage can throw outright in some contexts, so never let it take the
		// app down over a panel position.
		try {
			const stored = localStorage.getItem(KEY);
			if (stored === 'full' || stored === 'dialog') this.editorPresentation = stored;

			const start = localStorage.getItem(START_KEY);
			if (start === 'journal' || start === 'questions') this.startPage = start;

			const left = Number(localStorage.getItem(LEFT_KEY));
			if (left) this.sidebarWidth = clamp(left, SIDEBAR_MIN, SIDEBAR_MAX);

			const right = Number(localStorage.getItem(RIGHT_KEY));
			if (right) this.slotWidth = clamp(right, SLOT_MIN, SLOT_MAX);
		} catch {
			/* the default is fine */
		}
	}

	setSidebarWidth(px: number) {
		this.sidebarWidth = clamp(px, SIDEBAR_MIN, SIDEBAR_MAX);
		this.write(LEFT_KEY, String(this.sidebarWidth));
	}

	setSlotWidth(px: number) {
		this.slotWidth = clamp(px, SLOT_MIN, SLOT_MAX);
		this.write(RIGHT_KEY, String(this.slotWidth));
	}

	private write(key: string, value: string) {
		try {
			localStorage.setItem(key, value);
		} catch {
			/* it just will not persist */
		}
	}

	setEditorPresentation(value: EditorPresentation) {
		this.editorPresentation = value;
		try {
			localStorage.setItem(KEY, value);
		} catch {
			/* it just will not persist */
		}
	}

	setStartPage(value: StartPage) {
		this.startPage = value;
		this.write(START_KEY, value);
	}
}

function clamp(value: number, min: number, max: number) {
	return Math.min(max, Math.max(min, value));
}

export const prefs = new Prefs();
