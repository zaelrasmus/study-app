/**
 * Mirrors of the Rust domain types.
 *
 * Knowledge state is derived on the Rust side and arrives already computed, so
 * nothing in the frontend is allowed to invent or override it.
 */

export type NoteKind = 'long' | 'atomic';

/**
 * Axis 1. Exactly one is true at a time. Derived in Rust; the frontend never
 * computes it.
 */
export type RecallState =
	| 'raw'
	| 'draft'
	| 'recalled'
	| 'edited_since_recall'
	| 'failing';

export interface Note {
	id: string;
	kind: NoteKind;
	title: string;
	/** One line describing the concept. What the canvas and library render. */
	summary: string;
	body_json: string;
	/** Rendered HTML. What the assembled document displays. */
	body_html: string;
	body_text: string;
	content_hash: string;
	distilled_at: string | null;
	last_recall_at: string | null;
	last_checked_at: string | null;
	last_recall_failed_at: string | null;
	content_edited_at: string | null;
	content_hash_at_recall: string | null;
	/** The long note this one was distilled from. What the source pane shows. */
	source_note_id: string | null;
	/** The day this note is the page for, when it is one. */
	journal_day: string | null;
	created_at: string;
	updated_at: string;
	/** Axis 1, computed in Rust. Never assigned here. */
	state: RecallState;
	/** Axis 2: recalled, but never contrasted against a source. Coexists with
	 *  any recall state rather than replacing it. */
	never_contrasted: boolean;
	/**
	 * False for a note that never attempted memory. Computed in Rust, and the
	 * only gate on showing state anywhere: a note you only wanted to write is
	 * never labelled with how far it is from a pipeline it did not enter.
	 */
	visible: boolean;
}

export interface Topic {
	id: string;
	title: string;
	/** Study board or work board. Presentation only; never gates review. */
	study: boolean;
	created_at: string;
	updated_at: string;
	last_visited_at: string;
}

export type QuestionOrigin = 'recall_gap' | 'review_failure' | 'capture';

export type QuestionStatus =
	| 'open'
	| 'investigating'
	| 'resolved'
	| 'abandoned'
	| 'dissolved';

export interface Question {
	id: string;
	text: string;
	status: QuestionStatus;
	origin: QuestionOrigin;
	topic_id: string | null;
	source_note_id: string | null;
	answer_note_id: string | null;
	created_at: string;
	updated_at: string;
}

export interface Frame {
	id: string;
	topic_id: string;
	label: string;
	x: number;
	y: number;
	width: number;
	height: number;
	/** Explicit, visible on the frame, and the only thing that orders the document. */
	order_index: number;
	created_at: string;
	updated_at: string;
}

export type CanvasNodeKind =
	| 'note'
	| 'ink'
	| 'image'
	| 'pdf'
	| 'topic'
	| 'text'
	| 'link'
	| 'question';

export interface CanvasNode {
	id: string;
	topic_id: string;
	kind: CanvasNodeKind;
	x: number;
	y: number;
	width: number | null;
	height: number | null;
	z: number;
	note_id: string | null;
	target_topic_id: string | null;
	asset_id: string | null;
	/** The question a `question` node shows. */
	question_id: string | null;
	ink: string | null;
	/** The body of a `text` node: loose text with no note behind it. */
	text: string | null;
	/** Where a `link` node points. */
	url: string | null;
	/** A chosen card colour, per placement. Null means none. */
	color: string | null;
	/** Membership in a frame is what commits a node to the document. */
	frame_id: string | null;
	order_in_frame: number | null;
	created_at: string;
	updated_at: string;
}

/**
 * A drawn pointer between two cards on one board.
 *
 * Not a link between notes — those are derived from body text and rebuilt on
 * every save. This is the other claim: that these two cards, arranged this way,
 * on this board, belong together. It lives with the arrangement.
 */
export interface CanvasEdge {
	id: string;
	topic_id: string;
	source_id: string;
	target_id: string;
	label: string;
	created_at: string;
}

/**
 * A file you brought in: an image or a PDF, copied into the app's own
 * directory so a board does not break when you tidy a folder.
 *
 * Not a note. No title, no knowledge state, no card, and it never enters
 * review — it is what you think *about*, not what you have learned.
 */
export interface Asset {
	id: string;
	kind: 'image' | 'pdf';
	/** What it was called when it arrived. Shown, never used as a path. */
	file_name: string;
	mime: string;
	byte_size: number;
	/** Where the copy lives, relative to the asset directory. */
	rel_path: string;
	created_at: string;
}

export interface CanvasSnapshot {
	frames: Frame[];
	nodes: CanvasNode[];
	edges: CanvasEdge[];
	assets: Asset[];
	/** The questions placed on this board. */
	questions: Question[];
}

export interface DocumentSection {
	frame: Frame;
	notes: Note[];
}

/** A serialised freehand stroke, stored in `CanvasNode.ink`. */
export interface Ink {
	/** [x, y, pressure] triples, relative to the node origin. */
	points: [number, number, number][];
	size: number;
	thinning: number;
	smoothing: number;
	streamline: number;
}

/** The three band headers. Origin order *is* priority order. */
export const ORIGIN_LABEL: Record<QuestionOrigin, string> = {
	recall_gap: 'recall gaps',
	review_failure: 'review failures',
	capture: 'captures'
};

/** The per-item tag, shorter than the header it sits under. */
export const ORIGIN_TAG: Record<QuestionOrigin, string> = {
	recall_gap: 'from memory',
	review_failure: 'missed in review',
	capture: 'captured'
};

/** Why this band outranks the next one. Shown once, under the header. */
export const ORIGIN_WHY: Record<QuestionOrigin, string> = {
	recall_gap: 'Found while reconstructing something from memory.',
	review_failure: 'A card you missed.',
	capture: 'Written down without the system observing a gap.'
};

/**
 * The word shown on a card. English, always -- the interface language is
 * settled and state vocabulary gets no carve-out.
 */
export const STATE_LABEL: Record<RecallState, string> = {
	raw: 'not distilled',
	draft: 'draft',
	recalled: 'recalled',
	edited_since_recall: 'edited since recall',
	failing: 'failing'
};

/** One sentence of plain fact, never a reproach. */
export const STATE_MEANING: Record<RecallState, string> = {
	raw: 'Captured, not written into yet.',
	draft: 'Written with the source visible.',
	recalled: 'Reproduced from memory, unchanged since.',
	edited_since_recall: 'The text moved after you last reproduced it.',
	failing: 'Reproduced once, then missed in review.'
};

/** Only two states carry colour: the one worth reaching, and the one asking
 *  for action. Four colours would read as a grade. */
export const STATE_NEEDS_ACTION: Record<RecallState, boolean> = {
	raw: false,
	draft: false,
	recalled: false,
	edited_since_recall: true,
	failing: true
};

/** Four grades. Nothing here is a percentage or a streak. */
export type Grade = 'again' | 'hard' | 'good' | 'easy';

export const GRADE_LABEL: Record<Grade, string> = {
	again: 'Missed it',
	hard: 'Hard',
	good: 'Got it',
	easy: 'Easy'
};

/** A card is a projection of an atomic note, so it carries the note itself. */
export interface ReviewCard extends Note {
	topic_id: string | null;
	topic_title: string | null;
	reps: number;
}

/**
 * What a card shows as its face.
 *
 * Summary first — that is the one line that recovers the concept months later.
 * Then the title. Then the opening of the body, so a card is never blank.
 * Review is the one surface that ignores this and asks the title alone.
 */
export function cardFace(note: Pick<Note, 'summary' | 'title' | 'body_text'>): string {
	const summary = note.summary.trim();
	if (summary) return summary;

	const title = note.title.trim();
	if (title) return title;

	return note.body_text.trim().split('\n')[0] ?? '';
}
