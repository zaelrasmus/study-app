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
	import * as ipc from '$lib/ipc';
	import { chrome } from '$lib/state/chrome.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import { drag } from '$lib/state/drag.svelte';
	import { data } from '$lib/state/data.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { today as todayIso } from '$lib/date';
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
	let timer: ReturnType<typeof setTimeout> | undefined;

	const editor = createEditor({ onUpdate: () => schedule() });

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

	// Reloads the surface and the capture list whenever the day changes.
	$effect(() => {
		// A capture made from anywhere belongs to some day, and this is it.
		data.notes;
		const which = day;
		loading = true;

		Promise.all([ipc.journalPage(which), ipc.journalDay(which)]).then(([found, caught]) => {
			// A day switched during the round trip wins: this answer is stale, and
			// writing it into the editor would put one day's text on another.
			if (which !== current()) return;

			captures = caught;
			if (editor) {
				try {
					editor.commands.setContent(found?.body_json ? JSON.parse(found.body_json) : '');
				} catch {
					// A body we cannot parse is left alone rather than silently wiped.
				}
			}
			loading = false;
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
		clearTimeout(timer);
		timer = setTimeout(save, 700);
	}

	/** Writes now rather than on the timer. Used when leaving the day. */
	function flush() {
		clearTimeout(timer);
		if (!loading) save();
	}

	async function save() {
		if (!editor || loading) return;

		const which = day;
		saving = true;
		await ipc.saveJournalPage(
			which,
			JSON.stringify(editor.getJSON()),
			editor.getHTML(),
			editor.getText()
		);
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
				<Edra {editor}>
					<Edra.BubbleMenu />
					<Edra.Content class="edra-surface cursor-text *:outline-none" />
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
