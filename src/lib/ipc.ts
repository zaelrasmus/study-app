/**
 * Typed wrappers over the Tauri command surface.
 *
 * Everything the frontend knows about persistence goes through here, so there
 * is exactly one place to look when a command signature changes — and one place
 * to say what a write invalidates.
 *
 * Reads use `invoke`. Writes use `mutate`, which names the kinds of data it
 * disturbs so every screen showing that kind re-reads it. Putting that here
 * rather than at each call site is deliberate: a screen going stale because
 * someone forgot to fire a refresh is the bug this exists to prevent, and
 * leaving it to call sites reproduces it.
 */

import { invoke } from '@tauri-apps/api/core';
import { data, type DataKind } from './state/data.svelte';
import type {
	Asset,
	CanvasNode,
	CanvasNodeKind,
	CanvasEdge,
	CanvasSnapshot,
	DocumentSection,
	Frame,
	Note,
	NoteKind,
	Question,
	QuestionOrigin,
	QuestionStatus,
	ReviewCard,
	Grade,
	Topic
} from './types';

/**
 * A command that changes stored data.
 *
 * Identical to `invoke`, except that once it lands it marks what it touched.
 * Kinds are coarse on purpose — "some note changed" re-reads a list that is one
 * cheap query, whereas tracking rows would be a cache.
 */
function mutate<T>(kinds: readonly DataKind[], cmd: string, args?: Record<string, unknown>) {
	return invoke<T>(cmd, args).then((result) => {
		data.changed(kinds);
		return result;
	});
}

// -- topics ---------------------------------------------------------------

/** The sidebar. Recency, not structure. */
export const awakeTopics = (limit?: number) => invoke<Topic[]>('awake_topics', { limit });

export const allTopics = () => invoke<Topic[]>('all_topics');

export const createTopic = (title: string, id?: string) =>
	mutate<Topic>(['boards'], 'create_topic', { id, title });

export const renameTopic = (id: string, title: string) =>
	mutate<void>(['boards'], 'rename_topic', { id, title });

/** Opening a topic is the only thing that keeps it awake. */
export const openTopic = (id: string) => invoke<Topic>('open_topic', { id });

/**
 * Work board or study board. Presentation only, and non-destructive both ways:
 * turning study off hides the epistemic layer, it does not erase it.
 */
export const setTopicMode = (id: string, study: boolean) =>
	mutate<Topic>(['boards'], 'set_topic_mode', { id, study });

export const deleteTopic = (id: string) => mutate<void>(['boards', 'notes', 'questions'], 'delete_topic', { id });

// -- notes ----------------------------------------------------------------

export const createNote = (title: string, kind: NoteKind = 'long', id?: string) =>
	mutate<Note>(['notes'], 'create_note', { id, title, kind });

export const getNote = (id: string) => invoke<Note | null>('get_note', { id });

/** Notes that have never been placed on any canvas. */
export const inboxNotes = () => invoke<Note[]>('inbox_notes');

export const topicNotes = (topicId: string) => invoke<Note[]>('topic_notes', { topicId });

/**
 * Three projections of one body: JSON reloads the editor losslessly, HTML is
 * what the assembled document renders, and plaintext is what the content hash
 * is taken over — so restyling a note never costs you a verification.
 */
export const saveNote = (
	id: string,
	title: string,
	summary: string,
	bodyJson: string,
	bodyHtml: string,
	bodyText: string
) => mutate<Note>(['notes', 'tasks'], 'save_note', { id, title, summary, bodyJson, bodyHtml, bodyText });


export const setNoteKind = (id: string, kind: NoteKind) =>
	mutate<Note>(['notes', 'review'], 'set_note_kind', { id, kind });

/**
 * Step 2 of memory mode: save the blind rewrite.
 *
 * No `closedBook` argument — memory mode is the only route here, so a recall
 * is closed-book by construction. If the note was edited normally within the
 * last day the recall is still recorded, but it will not clear
 * `edited_since_recall`: reciting text you just wrote is not knowledge.
 */
export const recordRecall = (
	id: string,
	bodyJson: string,
	bodyHtml: string,
	bodyText: string
) => mutate<Note>(['notes', 'review'], 'record_recall', { id, bodyJson, bodyHtml, bodyText });

/** Links the long note being distilled from. What the source pane shows. */
export const setNoteSource = (id: string, sourceNoteId: string | null) =>
	mutate<Note>(['notes'], 'set_note_source', { id, sourceNoteId });

/** Step 4 of memory mode: the source was revealed and compared. Axis 2. */
export const recordContrast = (id: string) => mutate<Note>(['notes'], 'record_contrast', { id });

/** Accent-insensitive search over Spanish content. Keeps ñ intact. */
export const searchNotes = (query: string) => invoke<Note[]>('search_notes', { query });

/** What the library is filtered by. Scopes combine: a tag narrows a board. */
export interface LibraryFilter {
	query?: string;
	topicId?: string | null;
	unplacedOnly?: boolean;
	recalledOnly?: boolean;
	/** Tag membership, which the text search cannot see. */
	tagId?: string | null;
}

/**
 * The library: every note there is. Being on no canvas is the normal state of a
 * note, so unplaced is one filter here rather than a separate screen.
 */
export const libraryNotes = (filter: LibraryFilter = {}) =>
	invoke<Note[]>('library_notes', {
		filter: {
			query: filter.query ?? null,
			topic_id: filter.topicId ?? null,
			unplaced_only: filter.unplacedOnly ?? false,
			recalled_only: filter.recalledOnly ?? false,
			tag_id: filter.tagId ?? null
		}
	});

/** Which boards a note appears on. Boards import notes; they do not own them. */
export const noteBoards = (noteId: string) => invoke<Topic[]>('note_boards', { noteId });

export const deleteNote = (id: string) => mutate<void>(['notes', 'tasks', 'questions', 'review'], 'delete_note', { id });

/**
 * Distillation by drag: selected blocks become a note on the canvas.
 *
 * Copied, not moved — the long note keeps them. Untitled, so it arrives as a
 * long raw note; titling it is what makes it atomic.
 */
export const distilToCanvas = (
	topicId: string,
	sourceNoteId: string,
	bodyJson: string,
	bodyHtml: string,
	bodyText: string,
	x: number,
	y: number
) =>
	mutate<[Note, CanvasNode]>(['notes'], 'distil_to_canvas', {
		topicId,
		sourceNoteId,
		bodyJson,
		bodyHtml,
		bodyText,
		x,
		y
	});

// -- tags, mentions, tasks ------------------------------------------------

export interface Tag {
	id: string;
	name: string;
	fold: string;
	created_at: string;
}

export interface TagCount {
	id: string;
	name: string;
	fold: string;
	created_at: string;
	notes: number;
}

export interface DueTask {
	id: string;
	note_id: string;
	text: string;
	done: boolean;
	due_on: string | null;
	position: number;
	note_title: string;
	/** Set when it lives on a day's page, which has no title of its own. */
	journal_day: string | null;
}

export const allTags = () => invoke<TagCount[]>('all_tags');

export const noteTags = (noteId: string) => invoke<Tag[]>('note_tags', { noteId });

export const attachTag = (noteId: string, name: string) =>
	mutate<Tag>(['tags', 'notes'], 'attach_tag', { noteId, name });

/** Drops a tag from every note carrying it, then the tag. The notes survive. */
export const deleteTag = (id: string) => mutate<void>(['tags', 'notes'], 'delete_tag', { id });

/** Detaching also drops a tag nothing carries any more. */
export const detachTag = (noteId: string, tagId: string) =>
	mutate<void>(['tags', 'notes'], 'detach_tag', { noteId, tagId });

/** Notes that mention this one. Derived from the body, never edited directly. */
export const noteBacklinks = (noteId: string) => invoke<Note[]>('note_backlinks', { noteId });

/**
 * Every open task, dated or not. What the Tasks screen shows.
 *
 * A task with no `due:` used to be indexed and then reachable from nowhere,
 * which read as the app losing your writing. The screen shows all of them; the
 * panel is what stays one day wide.
 */
export const allTasks = () => invoke<DueTask[]>('all_tasks');

/**
 * Ticks or unticks a task, in the note that owns it.
 *
 * The body stays the truth: this edits the document and rebuilds the index from
 * it. Ticking is not writing — the text is unchanged — so it costs no recall.
 */
export const setTaskDone = (taskId: string, done: boolean) =>
	mutate<void>(['tasks', 'notes'], 'set_task_done', { taskId, done });

/** Tasks due on one day. What the panel shows. */
export const tasksDue = (day: string) => invoke<DueTask[]>('tasks_due', { day });

// -- journal --------------------------------------------------------------

/**
 * One day's captures. A filter over notes, never a container — appending to
 * today just creates another capture, which is what keeps capture free of any
 * decision about where a thought belongs.
 */
export const journalDay = (day: string) => invoke<Note[]>('journal_day', { day });

/**
 * The page you write on for a day. Null until you have written something —
 * visiting a date never manufactures an empty note.
 */
export const journalPage = (day: string) => invoke<Note | null>('journal_page', { day });

/**
 * Writes a day's page, creating it on the first keystroke.
 *
 * Any day already past is writable: yesterday is a record you may correct. What
 * the journal refuses is writing *forward*, and that is enforced by the week
 * strip, which cannot select a future day.
 */
export const saveJournalPage = (
	day: string,
	bodyJson: string,
	bodyHtml: string,
	bodyText: string
) => mutate<Note>(['notes', 'tasks'], 'save_journal_page', { day, bodyJson, bodyHtml, bodyText });

/** Which days in a range have anything on them, for the week strip. */
export const journalMarks = (from: string, to: string) =>
	invoke<string[]>('journal_marks', { from, to });

// -- questions ------------------------------------------------------------

/** Already ordered by origin. Do not re-sort in the UI. */
export const questionQueue = (topicId?: string | null) =>
	invoke<Question[]>('question_queue', { topicId: topicId ?? null });

/**
 * Open questions attached to no board.
 *
 * A doubt caught while writing in the journal has no topic and is still a
 * doubt, so it is reachable as its own scope rather than hidden everywhere.
 */
export const unattachedQuestions = () => invoke<Question[]>('unattached_questions');

export const createQuestion = (
	text: string,
	origin: QuestionOrigin,
	topicId?: string | null,
	sourceNoteId?: string | null
) =>
	mutate<Question>(['questions'], 'create_question', {
		text,
		origin,
		topicId: topicId ?? null,
		sourceNoteId: sourceNoteId ?? null
	});

/** Closing requires the note that answers it. */
export const resolveQuestion = (id: string, answerNoteId: string) =>
	mutate<Question>(['questions', 'notes'], 'resolve_question', { id, answerNoteId });

/** Abandon and dissolve. One call, never behind a confirmation. */
export const setQuestionStatus = (id: string, status: QuestionStatus) =>
	mutate<Question>(['questions'], 'set_question_status', { id, status });

export const setQuestionTopic = (id: string, topicId: string | null) =>
	mutate<Question>(['questions'], 'set_question_topic', { id, topicId });

export const openQuestionCount = (topicId?: string | null) =>
	invoke<number>('open_question_count', { topicId: topicId ?? null });

// -- review ---------------------------------------------------------------

/** Today's queue, capped at a day's worth. The backlog is never exposed. */
export const reviewQueue = () => invoke<ReviewCard[]>('review_queue');

/** Bounded work, so it is safe to show in the sidebar. */
export const reviewDueToday = () => invoke<number>('review_due_today');

export const gradeCard = (noteId: string, grade: Grade) =>
	mutate<void>(['review'], 'grade_card', { noteId, grade });

/** A drifted card leaves the queue by being acted on, not by being graded. */
export const resolveDriftedCard = (noteId: string, stillHolds: boolean) =>
	mutate<void>(['review', 'notes'], 'resolve_drifted_card', { noteId, stillHolds });

// -- canvas ---------------------------------------------------------------

export const canvasSnapshot = (topicId: string) =>
	invoke<CanvasSnapshot>('canvas_snapshot', { topicId });

export const createFrame = (
	topicId: string,
	label: string,
	x: number,
	y: number,
	width: number,
	height: number,
	id?: string
) => mutate<Frame>(['boards'], 'create_frame', { id, topicId, label, x, y, width, height });

/** Geometry only: this can never change document order. */
export const moveFrame = (id: string, x: number, y: number, width: number, height: number) =>
	mutate<void>(['boards'], 'move_frame', { id, x, y, width, height });

export const labelFrame = (id: string, label: string) => mutate<void>(['boards'], 'label_frame', { id, label });

/** The one operation that does change document order. */
export const reorderFrames = (topicId: string, ids: string[]) =>
	mutate<void>(['boards'], 'reorder_frames', { topicId, ids });

export const deleteFrame = (id: string) => mutate<void>(['boards'], 'delete_frame', { id });

export const addNoteNode = (topicId: string, noteId: string, x: number, y: number, id?: string) =>
	mutate<CanvasNode>(['notes', 'boards'], 'add_note_node', { id, topicId, noteId, x, y });

/** Capture straight onto the canvas: one gesture, one round trip. */
export const createNoteOnCanvas = (topicId: string, title: string, x: number, y: number) =>
	mutate<[Note, CanvasNode]>(['notes', 'boards'], 'create_note_on_canvas', { topicId, title, x, y });

/**
 * Puts a board on a board, as a card that opens it.
 *
 * **Nothing calls this yet.** The backend is finished and covered by tests
 * (including the cycle check — a board cannot end up inside itself), but no
 * gesture in the canvas creates one. It is here rather than deleted because the
 * missing half is the UI, not the model.
 */
export const addTopicNode = (
	topicId: string,
	targetTopicId: string,
	x: number,
	y: number,
	id?: string
) => mutate<CanvasNode>(['boards'], 'add_topic_node', { id, topicId, targetTopicId, x, y });

/** Loose text on a board: no title, no state, no card, no note behind it. */
export const addTextNode = (topicId: string, text: string, x: number, y: number, id?: string) =>
	mutate<CanvasNode>(['boards'], 'add_text_node', { id, topicId, text, x, y });

export const setNodeText = (id: string, text: string) =>
	mutate<void>(['boards'], 'set_node_text', { id, text });

/** Turns loose text into a note in place: same node, same spot, same board. */
export const promoteTextNode = (id: string) =>
	mutate<[Note, CanvasNode]>(['notes', 'boards'], 'promote_text_node', { id });

/** Draws an arrow between two cards. Idempotent on the ordered pair. */
export const connectNodes = (sourceId: string, targetId: string, id?: string) =>
	mutate<CanvasEdge>(['boards'], 'connect_nodes', { id, sourceId, targetId });

export const labelEdge = (id: string, label: string) =>
	mutate<void>(['boards'], 'label_edge', { id, label });

/** One click, no confirmation. An arrow is an arrangement, not a document. */
export const disconnectNodes = (id: string) => mutate<void>(['boards'], 'disconnect_nodes', { id });

/**
 * Brings a file onto a board: an image or a PDF, copied in and placed.
 *
 * Copied rather than referenced, so a board does not break when you move or
 * tidy the original. Images and PDFs only — anything the webview cannot render
 * inline would arrive as a card you could not look at.
 */
export const importAsset = (topicId: string, path: string, x: number, y: number) =>
	mutate<[Asset, CanvasNode]>(['boards'], 'import_asset', { topicId, path, x, y });

/** Where the copies live. Turned into a renderable URL with `convertFileSrc`. */
export const assetDir = () => invoke<string>('asset_dir');

/**
 * Puts an open question on a board, beside the cards that might answer it.
 *
 * Placement is membership: a doubt caught while writing has no board, and
 * dropping it on a canvas is what gives it one — which is what makes it findable
 * from that board afterwards.
 */
export const addQuestionNode = (
	topicId: string,
	questionId: string,
	x: number,
	y: number,
	id?: string
) => mutate<CanvasNode>(['questions', 'boards'], 'add_question_node', { id, topicId, questionId, x, y });

/** A page on the web, placed on the board. Nothing is copied. */
export const addLinkNode = (topicId: string, url: string, x: number, y: number, id?: string) =>
	mutate<CanvasNode>(['boards'], 'add_link_node', { id, topicId, url, x, y });

export const addInkNode = (topicId: string, ink: string, x: number, y: number, id?: string) =>
	mutate<CanvasNode>(['boards'], 'add_ink_node', { id, topicId, ink, x, y });

export const addAssetNode = (
	topicId: string,
	assetId: string,
	kind: CanvasNodeKind,
	x: number,
	y: number,
	id?: string
) => mutate<CanvasNode>(['boards'], 'add_asset_node', { id, topicId, assetId, kind, x, y });

export const moveNode = (
	id: string,
	x: number,
	y: number,
	width?: number | null,
	height?: number | null
) => mutate<void>(['boards'], 'move_node', { id, x, y, width: width ?? null, height: height ?? null });

/** Committing a node to the document, or taking it back out. */
export const setNodeFrame = (id: string, frameId: string | null, orderInFrame: number | null) =>
	mutate<void>(['boards'], 'set_node_frame', { id, frameId, orderInFrame });

/** A chosen card colour, per placement. Null clears it. */
export const setNodeColor = (id: string, color: string | null) =>
	mutate<void>(['boards'], 'set_node_color', { id, color });

export const deleteNode = (id: string) => mutate<void>(['notes', 'boards'], 'delete_node', { id });

/** The document, assembled from the arrangement. */
export const topicDocument = (topicId: string) =>
	invoke<DocumentSection[]>('topic_document', { topicId });
