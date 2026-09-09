/**
 * When stored data changed, and what kind.
 *
 * Every screen loads inside an `$effect` that depends on its own inputs — the
 * board in view, the day, the chosen scope. Nothing else touches those, so a
 * write from somewhere else never re-ran the load: capture a question with a
 * board open and its panel kept showing the list from before, until you closed
 * and reopened it. That was true of nearly every pair of surfaces in the app,
 * not just that one.
 *
 * This is the missing signal. A reader names the kinds it depends on:
 *
 * ```ts
 * $effect(() => {
 *   data.questions;              // re-run when any question changes
 *   ipc.questionQueue(topicId).then((found) => (questions = found));
 * });
 * ```
 *
 * and every mutating command in `ipc.ts` marks its kinds automatically. Doing
 * it there rather than at each call site is the point: a signal you have to
 * remember to send is a signal that will be forgotten, and this is exactly the
 * bug that produced.
 *
 * It is deliberately coarse. "Some question changed" is enough to re-read a
 * list that is already one cheap SQLite query; tracking which *row* changed
 * would be a cache, and a cache is a second copy of the truth.
 */

export type DataKind = 'notes' | 'questions' | 'tasks' | 'tags' | 'boards' | 'review';

/**
 * Long enough to swallow an autosave storm, short enough to feel immediate.
 *
 * The editor saves every 700ms while you type, and each save touches notes,
 * tasks and links. Without this, every list in the window would refetch on that
 * cadence while you wrote a sentence.
 */
const QUIET_MS = 180;

class Data {
	notes = $state(0);
	questions = $state(0);
	tasks = $state(0);
	tags = $state(0);
	boards = $state(0);
	review = $state(0);

	#pending = new Set<DataKind>();
	#timer: ReturnType<typeof setTimeout> | undefined;

	/** Called by `ipc.ts` after a write lands. Coalesced, never immediate. */
	changed(kinds: readonly DataKind[]) {
		for (const kind of kinds) this.#pending.add(kind);

		clearTimeout(this.#timer);
		this.#timer = setTimeout(() => this.#flush(), QUIET_MS);
	}

	/** For the rare case that has to be reflected before the next paint. */
	flushNow() {
		clearTimeout(this.#timer);
		this.#flush();
	}

	#flush() {
		for (const kind of this.#pending) this[kind] += 1;
		this.#pending.clear();
	}
}

export const data = new Data();
