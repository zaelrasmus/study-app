/**
 * Pointer-based dragging.
 *
 * HTML5 drag-and-drop does not work here. Tauri's webview claims the native
 * drag-drop handler for file drops (`dragDropEnabled` defaults to true), so
 * `dragstart` fires and then nothing else ever happens — no `dragover`, no
 * `drop`, and no error. Turning that config off would restore HTML5 DnD, but
 * pointer events are the better answer anyway: they work regardless of the
 * setting, they give us a real ghost that follows the cursor, and they behave
 * correctly inside a canvas that pans and zooms.
 *
 * Drop targets are found by hit-testing on release rather than by registering
 * rectangles, so a target can appear, move or scroll mid-drag and still work.
 */

export type DragPayload =
	| { kind: 'note'; noteId: string; label: string }
	| { kind: 'frame'; frameId: string; label: string }
	/** An open question, on its way to a board that might answer it. */
	| { kind: 'question'; questionId: string; label: string }
	/** A selection pulled out of an editor, carried as a real document. */
	| { kind: 'blocks'; json: string; html: string; text: string; label: string };

export interface DropContext {
	clientX: number;
	clientY: number;
	/** The element carrying `data-drop-zone`, so a target can read its own ids. */
	target: HTMLElement;
}

/** Movement before a press becomes a drag, so clicks still land. */
const THRESHOLD = 4;

class Drag {
	/** Non-null only once the threshold is crossed. */
	payload = $state<DragPayload | null>(null);
	x = $state(0);
	y = $state(0);
	/** The zone under the cursor right now, for highlighting. */
	over = $state<string | null>(null);

	#handlers = new Map<string, (payload: DragPayload, ctx: DropContext) => void>();
	#pending: { payload: DragPayload; startX: number; startY: number } | null = null;

	/** A drop target claims a name; the element carries `data-drop-zone`. */
	register(zone: string, handler: (payload: DragPayload, ctx: DropContext) => void) {
		this.#handlers.set(zone, handler);
		return () => this.#handlers.delete(zone);
	}

	/**
	 * Call from `onpointerdown`. Nothing visible happens until the pointer has
	 * actually moved, so this is safe to attach to things that are also clickable.
	 */
	begin(payload: DragPayload, event: PointerEvent) {
		if (event.button !== 0) return;

		this.#pending = { payload, startX: event.clientX, startY: event.clientY };
		this.x = event.clientX;
		this.y = event.clientY;

		window.addEventListener('pointermove', this.#move);
		window.addEventListener('pointerup', this.#up);
		window.addEventListener('pointercancel', this.#cancel);
	}

	#move = (event: PointerEvent) => {
		this.x = event.clientX;
		this.y = event.clientY;

		if (this.#pending && !this.payload) {
			const dx = event.clientX - this.#pending.startX;
			const dy = event.clientY - this.#pending.startY;
			if (Math.hypot(dx, dy) < THRESHOLD) return;
			this.payload = this.#pending.payload;
		}

		if (this.payload) this.over = this.#zoneAt(event.clientX, event.clientY)?.zone ?? null;
	};

	#up = (event: PointerEvent) => {
		const payload = this.payload;
		this.#teardown();
		if (!payload) return;

		const hit = this.#zoneAt(event.clientX, event.clientY);
		if (!hit) return;

		this.#handlers.get(hit.zone)?.(payload, {
			clientX: event.clientX,
			clientY: event.clientY,
			target: hit.element
		});
	};

	#cancel = () => this.#teardown();

	#teardown() {
		this.#pending = null;
		this.payload = null;
		this.over = null;
		window.removeEventListener('pointermove', this.#move);
		window.removeEventListener('pointerup', this.#up);
		window.removeEventListener('pointercancel', this.#cancel);
	}

	/**
	 * The ghost is `pointer-events: none`, so `elementFromPoint` returns what is
	 * genuinely underneath rather than the thing being dragged.
	 */
	#zoneAt(x: number, y: number): { zone: string; element: HTMLElement } | null {
		const el = document.elementFromPoint(x, y);
		const zoneEl = (el as HTMLElement | null)?.closest<HTMLElement>('[data-drop-zone]');
		if (!zoneEl) return null;

		const zone = zoneEl.dataset.dropZone;
		return zone ? { zone, element: zoneEl } : null;
	}
}

export const drag = new Drag();
