<script lang="ts">
	/**
	 * A day, in the middle of the window.
	 *
	 * The week strip sits in the top bar directly above the day it selects, and
	 * the day's page sits under it. That adjacency is the point of moving this
	 * out of the right slot: picking a date and reading what is on it are one
	 * act, so they belong on one axis.
	 *
	 * **The page is a real writing surface**, not a capture box. You write into
	 * a day the way you write into any note — same editor, same formatting, same
	 * autosave. It is created on the first keystroke, so browsing a year of
	 * dates leaves nothing behind.
	 *
	 * Any past day is writable: yesterday is a record you may correct. What the
	 * journal refuses is writing *forward* — a journal you can fill in ahead is
	 * a planner, and a planner accumulates commitments that grow while you
	 * sleep. That is enforced by the strip, which cannot select a future day.
	 *
	 * Below the page, the day's **captures** are still just a filter over notes
	 * by creation date. Nothing groups them and there is no day-container: the
	 * page is one more note that happens to know which day it is for.
	 */
	import { onDestroy } from 'svelte';
	import * as ipc from '$lib/ipc';
	import { chrome } from '$lib/state/chrome.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import { drag } from '$lib/state/drag.svelte';
	import { data } from '$lib/state/data.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { today as todayIso } from '$lib/date';
	import { headingsOf, outlineKey, goToHeading } from '$lib/outline';
	import { keepRoomBelow } from '$lib/room';
	import { setTaskChecked } from '$lib/tasks';
	import type { Note } from '$lib/types';
	import { cardFace } from '$lib/types';
	import WeekBar from '$lib/components/WeekBar.svelte';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import { Edra, createEditor } from '$lib/components/edra/shadcn/index.js';
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import ChevronIcon from '@lucide/svelte/icons/chevron-right';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';

	const today = todayIso();

	/** In the URL, so a day is a place you can go back to. */
	let day = $state(page.url.searchParams.get('d') ?? today);
	let captures = $state<Note[]>([]);
	let editingNoteId = $state<string | null>(null);
	/** Folded by default: the page is what you came here to write. */
	let showCaptures = $state(false);

	/** Suppresses the autosave while the editor is being filled from the DB. */
	let loading = $state(true);
	let saving = $state(false);
	/**
	 * Whether the surface holds anything the database has not been told about.
	 *
	 * Without this the page wrote itself out on every teardown, changed or not —
	 * and a document written back unchanged is not harmless. It is the editor's
	 * copy, which goes stale the moment anything else edits the note, so merely
	 * walking away from a day could undo a task ticked from the panel.
	 */
	let dirty = false;
	/** The note behind the day, once there is one. Null until the page exists. */
	let pageNoteId = $state<string | null>(null);
	let timer: ReturnType<typeof setTimeout> | undefined;

	/**
	 * Set by any keystroke, cleared whenever a load starts.
	 *
	 * A load in flight answers with the day as it was *before* the words you are
	 * typing while it is out, so its answer arrives already stale.
	 */
	let touched = false;

	const editor = createEditor({
		onUpdate: () => {
			touched = true;
			publishOutline();
			schedule();
		}
	});

	// The line you are writing never sits flat against the bottom of the window.
	keepRoomBelow(editor);

	/**
	 * While this page is open, it owns the day's document.
	 *
	 * Anything else that wants to tick a checkbox in it comes through here
	 * rather than writing the body to the database, which this editor would
	 * overwrite from its own copy the next time it saved.
	 *
	 * Re-registered when a note opened over the page closes: that note claims
	 * the same slot if it happens to be this very day's page.
	 */
	$effect(() => {
		editingNoteId;
		const id = pageNoteId;
		if (!id) return;
		return slot.holdDocument(id, (index, done) => setTaskChecked(editor, index, done));
	});

	/**
	 * The Outline panel, kept in step with the page.
	 *
	 * Recomputed on every update but only published when the shape of the
	 * document actually changed, so typing inside a paragraph does not rebuild
	 * the panel on every keystroke.
	 *
	 * `force` is for taking the outline back: a note opened over this page
	 * publishes its own headings, and when it closes the page underneath has to
	 * say so again even though nothing here changed.
	 */
	let lastOutline = '';

	function publishOutline(force = false) {
		if (!editor) return;
		const found = headingsOf(editor);
		const key = outlineKey(found);
		if (!force && key === lastOutline) return;
		lastOutline = key;
		slot.setOutline(found, (pos) => goToHeading(editor, pos));
	}

	const weekday = $derived(
		new Date(`${day}T00:00:00`).toLocaleDateString(undefined, { weekday: 'long' })
	);

	const heading = $derived(
		new Date(`${day}T00:00:00`).toLocaleDateString(undefined, {
			day: 'numeric',
			month: 'long',
			year: 'numeric'
		})
	);

	function pick(next: string) {
		if (next === day) return;
		// Whatever is in the editor belongs to the day being left, so it is
		// written before the surface is refilled with another one.
		flush();
		day = next;

		const url = new URL(page.url);
		if (next === today) url.searchParams.delete('d');
		else url.searchParams.set('d', next);
		replaceState(url, {});
	}

	/**
	 * The day's text, refilled when you change day — and only then.
	 *
	 * Deliberately *not* subscribed to `data.notes`. Saving the page marks notes
	 * changed, so a reader watching that would reload the very day it had just
	 * written and pour the answer back into the editor mid-sentence. `setContent`
	 * replaces the whole document, and ProseMirror maps the caret to the end of
	 * the replacement: you paused, the autosave fired, and the next word you
	 * typed landed on the last line.
	 *
	 * Nothing but this screen writes a journal page, so there is no second writer
	 * to hear about in the first place.
	 */
	$effect(() => {
		const which = day;
		loading = true;
		touched = false;
		dirty = false;
		// Belongs to the day being left until the new one answers.
		pageNoteId = null;

		ipc.journalPage(which).then((found) => {
			// A day switched during the round trip wins: this answer is stale, and
			// writing it into the editor would put one day's text on another.
			if (which !== current()) return;
			// So does a keystroke. The answer predates it, and refilling the surface
			// would swallow whatever was typed while it was in flight.
			if (editor && !touched) {
				try {
					editor.commands.setContent(found?.body_json ? JSON.parse(found.body_json) : '', {
						// Filling the surface from the database is not an edit, and must
						// not reach the autosave as one.
						emitUpdate: false
					});
				} catch {
					// A body we cannot parse is left alone rather than silently wiped.
				}
			}
			pageNoteId = found?.id ?? null;
			// Filling the surface emits no update, so the outline is published by
			// hand rather than arriving through `onUpdate`.
			publishOutline(true);
			loading = false;
		});
	});

	/**
	 * The captures under the page, which *do* follow every write: a doubt caught
	 * from the canvas or the command bar belongs to some day, and this is it.
	 *
	 * Re-reading a list costs one query and disturbs nothing. It is only the
	 * editor that cannot be reloaded out from under you.
	 */
	$effect(() => {
		data.notes;
		const which = day;
		ipc.journalDay(which).then((caught) => {
			if (which === current()) captures = caught;
		});
	});

	/** Read outside the tracking context, to compare against a stale response. */
	function current() {
		return day;
	}

	// The Tasks panel takes its date from here, so arrowing back through the
	// week moves the panel with the page instead of always meaning "today".
	$effect(() => {
		slot.day = day;
		return () => (slot.day = null);
	});

	/**
	 * Whoever is on top owns the outline, so the page takes it back when a note
	 * opened over it closes.
	 *
	 * In a microtask because the note's teardown clears the outline, and that
	 * happens while this effect is being re-run — publishing synchronously would
	 * put the page's headings up and have them wiped a moment later.
	 */
	$effect(() => {
		editingNoteId;
		queueMicrotask(() => publishOutline(true));
	});


	// The inspector borrows this screen's opener, so a backlink is followable
	// from the panel without the panel needing to know how notes get opened.
	$effect(() => {
		slot.openNote = (id: string) => (editingNoteId = id);
		return () => (slot.openNote = undefined);
	});

	$effect(() => chrome.claim(bar));

	/** A capture you no longer want. Deleting is everywhere, so it asks. */
	function captureMenu(event: MouseEvent, note: Note) {
		menu.show(event, [
			{ label: 'Open', onpick: () => (editingNoteId = note.id) },
			{
				label: 'Delete note',
				destructive: true,
				onpick: async () => {
					const face = cardFace(note) || 'Untitled';
					if (!confirm(`Delete "${face}"? This cannot be undone.`)) return;
					captures = captures.filter((n) => n.id !== note.id);
					await ipc.deleteNote(note.id);
				}
			}
		]);
	}

	/** The body under the heading, so a card says more than a row could. */
	function excerpt(note: Note) {
		const face = cardFace(note);
		const body = note.body_text.trim();
		const rest = body.startsWith(face) ? body.slice(face.length) : body;
		return rest.replace(/\s+/g, ' ').trim().slice(0, 160);
	}

	function schedule() {
		if (loading) return;
		dirty = true;
		clearTimeout(timer);
		timer = setTimeout(save, 700);
	}

	/** Writes now rather than on the timer. Used when leaving the day. */
	function flush() {
		clearTimeout(timer);
		// Only if there is something to write. Never worth an error either: the
		// words are on screen, and the next keystroke schedules another attempt.
		if (!loading && dirty) save().catch(() => {});
	}

	/**
	 * The last sentence, on the way out.
	 *
	 * The autosave waits 700ms for you to stop typing, so leaving the journal
	 * inside that window — clicking Library, opening a board — left the words
	 * after the last save unwritten, and coming back showed the day without
	 * them. Tearing down is the one moment guaranteed to happen.
	 */
	onDestroy(() => {
		flush();
		// Nothing in the centre is a document any more.
		slot.setOutline([], () => {});
	});

	async function save() {
		if (!editor || loading) return;

		const which = day;
		saving = true;
		// Cleared before the write, not after: a keystroke landing while this is
		// in flight has to leave the page dirty, or it would be dropped.
		dirty = false;
		const saved = await ipc.saveJournalPage(
			which,
			JSON.stringify(editor.getJSON()),
			editor.getHTML(),
			editor.getText()
		);
		// The first keystroke of a day is what creates its page.
		if (which === current()) pageNoteId = saved.id;
		saving = false;
	}
</script>

<svelte:head><title>Journal — study</title></svelte:head>
<svelte:window onbeforeunload={flush} />

{#snippet bar()}
	<WeekBar {day} onpick={pick} />
{/snippet}

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-2xl flex-col px-10 py-12">
		<header class="flex items-baseline gap-3">
			<div class="min-w-0 grow">
				<p class="label mb-1.5">{weekday}</p>
				<h1 class="text-[26px] font-semibold tracking-tight">{heading}</h1>
			</div>
			<!-- The only feedback autosave needs: it is either settled or it is not. -->
			<span
				class="label shrink-0 transition-opacity"
				style="opacity: {saving ? 0.5 : 0}"
			>
				saving
			</span>
		</header>

		<!-- The page. Same editor as any note, on any day already past. -->
		<div class="relative mt-6 min-h-40">
			{#if editor}
				<!-- `pb` is the run-off at the foot of the page, and it is part of
				     the writing surface rather than a margin under it: the padding
				     is inside the contenteditable, so clicking down there puts the
				     caret at the end instead of doing nothing.

				     Without it the last line you type sits jammed against the
				     bottom edge of the window, which is exactly where the messy
				     end of a day's thinking happens. Enough room to keep writing
				     near the middle of the screen. -->
				<Edra {editor}>
					<Edra.BubbleMenu />
					<Edra.Content class="edra-surface cursor-text pb-[45vh] *:outline-none" />
					<Edra.DragHandle />
				</Edra>
			{/if}
		</div>

		{#if captures.length > 0}
			<section class="mt-12 flex flex-col">
				<!-- Caught during the day, wherever you were. Not part of the page:
				     the page is what you wrote, these are what you grabbed.
				
				     Folded away by default. The page is the thing you came here to
				     write, and a grid of unrelated captures sitting under it turns
				     the screen into a pile. -->
				<button
					type="button"
					onclick={() => (showCaptures = !showCaptures)}
					class="hover:bg-accent -mx-2 flex items-center gap-2 self-start rounded-md px-2 py-1 transition-colors"
				>
					<ChevronIcon
						class="size-3 opacity-35 transition-transform"
						style="transform: rotate({showCaptures ? 90 : 0}deg)"
					/>
					<span class="label">Captured</span>
					<span class="font-mono text-[10px] tabular-nums opacity-30">{captures.length}</span>
				</button>

				<!-- Cards, like the library: a capture is a thing, and it should
				     look like the same thing wherever you meet it. -->
				{#if showCaptures}
				<div
					class="mt-3 grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-2.5"
					transition:slide={{ duration: 160, easing: cubicOut }}
				>
					{#each captures as note (note.id)}
						<button
							type="button"
							onclick={() => (editingNoteId = note.id)}
							onpointerdown={(e) =>
								drag.begin({ kind: 'note', noteId: note.id, label: cardFace(note) }, e)}
							oncontextmenu={(e) => captureMenu(e, note)}
							title="Click to open, drag onto a board to place it"
							class="group bg-card hover:border-foreground/25 flex max-h-56 min-h-20 cursor-grab
								touch-none flex-col gap-2 overflow-hidden rounded-lg border p-3.5 text-left
								transition-colors active:cursor-grabbing"
							style="border-color: var(--hairline)"
						>
							<div class="flex items-start gap-2">
								<StateMark
									state={note.state}
									visible={note.visible}
									neverContrasted={note.never_contrasted}
									class="mt-[5px] shrink-0"
								/>
								<span class="min-w-0 grow text-[13px] leading-snug font-medium">
									{cardFace(note) || 'Untitled'}
								</span>
								<span
									class="shrink-0 pt-[3px] font-mono text-[9.5px] tabular-nums opacity-0 transition-opacity group-hover:opacity-30"
								>
									{new Date(note.created_at).toLocaleTimeString(undefined, {
										hour: '2-digit',
										minute: '2-digit'
									})}
								</span>
							</div>

							{#if excerpt(note)}
								<p class="text-muted-foreground min-h-0 overflow-hidden text-[11.5px] leading-relaxed">
									{excerpt(note)}
								</p>
							{/if}
						</button>
					{/each}
				</div>
				{/if}
			</section>
		{/if}
	</div>
</div>

{#if editingNoteId}
	{#key editingNoteId}
		<NoteEditor
		noteId={editingNoteId}
		onclose={() => (editingNoteId = null)}
		onsaved={() => ipc.journalDay(day).then((found) => (captures = found))}
	/>
	{/key}
{/if}
