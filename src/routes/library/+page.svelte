<script lang="ts">
	/**
	 * Every note there is.
	 *
	 * Boards do not own notes — they import them. So this is the whole corpus,
	 * and a board is composed by pulling in what already exists rather than
	 * retyping it. That is the difference between a canvas app and a note app
	 * with a canvas bolted on.
	 *
	 * It is a full screen rather than a panel because that is what it is for:
	 * seeing everything at once, at a size where the writing is legible. The
	 * narrow list in the right slot survives only on a board, where its job is
	 * to be a drag source and nothing else.
	 *
	 * Being on no canvas is the normal state of a note. It is a filter here, not
	 * a condition to be fixed, and it carries no count of its own: a tally of
	 * everything you have ever written is not pending work, it is size.
	 */
	import * as ipc from '$lib/ipc';
	import { slot } from '$lib/state/slot.svelte';
	import { chrome } from '$lib/state/chrome.svelte';
	import { drag } from '$lib/state/drag.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { data } from '$lib/state/data.svelte';
	import type { Note } from '$lib/types';
	import { cardFace } from '$lib/types';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import SearchIcon from '@lucide/svelte/icons/search';
	import XIcon from '@lucide/svelte/icons/x';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';

	type Scope = 'all' | 'inbox' | 'recalled';

	const SCOPES: { id: Scope; label: string; hint: string }[] = [
		{ id: 'all', label: 'All', hint: 'Everything you have written' },
		{ id: 'inbox', label: 'Unplaced', hint: 'Captured, not on any board' },
		{ id: 'recalled', label: 'Recalled', hint: 'Reproduced from memory at least once' }
	];

	let query = $state('');
	let scope = $state<Scope>('all');
	let notes = $state<Note[]>([]);
	let tags = $state<ipc.TagCount[]>([]);
	let editingNoteId = $state<string | null>(null);

	/** In the URL, so a tag-filtered library is a place you can return to. */
	let tagId = $state<string | null>(page.url.searchParams.get('tag'));

	const tag = $derived(tags.find((t) => t.id === tagId) ?? null);

	$effect(() => {
		data.tags;
		ipc.allTags().then((found) => (tags = found));
	});

	$effect(() => {
		data.notes;
		data.boards;
		ipc
			.libraryNotes({
				query: query || undefined,
				unplacedOnly: scope === 'inbox',
				recalledOnly: scope === 'recalled',
				tagId
			})
			.then((found) => (notes = found));
	});


	// The inspector borrows this screen's opener, so a backlink is followable
	// from the panel without the panel needing to know how notes get opened.
	$effect(() => {
		slot.openNote = (id: string) => (editingNoteId = id);
		return () => (slot.openNote = undefined);
	});

	$effect(() => chrome.claim(bar));

	function clearTag() {
		tagId = null;
		const url = new URL(page.url);
		url.searchParams.delete('tag');
		replaceState(url, {});
	}

	/**
	 * Dragging a note onto a board is how a board gets composed.
	 *
	 * Pointer events rather than HTML5 drag-and-drop: Tauri's webview claims the
	 * native drag handler for file drops, so `dragstart` fires and nothing else
	 * ever does.
	 */
	function grab(event: PointerEvent, note: Note) {
		drag.begin({ kind: 'note', noteId: note.id, label: cardFace(note) }, event);
	}

	/**
	 * Right-clicking a card.
	 *
	 * Deleting from here is deleting everywhere — the library *is* everywhere —
	 * so it asks, and it is the same weight as deleting from a board.
	 */
	function cardMenu(event: MouseEvent, note: Note) {
		menu.show(event, [
			{ label: 'Open', onpick: () => (editingNoteId = note.id) },
			{ label: 'Delete note', destructive: true, onpick: () => remove(note) }
		]);
	}

	async function remove(note: Note) {
		const face = cardFace(note) || 'Untitled';
		if (!confirm(`Delete "${face}"? This cannot be undone.`)) return;

		notes = notes.filter((n) => n.id !== note.id);
		await ipc.deleteNote(note.id);
	}

	/** The body under the heading, trimmed to something a card can hold. */
	function excerpt(note: Note) {
		const face = cardFace(note);
		const body = note.body_text.trim();
		const rest = body.startsWith(face) ? body.slice(face.length) : body;
		return rest.replace(/\s+/g, ' ').trim().slice(0, 220);
	}
</script>

<svelte:head><title>Library — study</title></svelte:head>

{#snippet bar()}
	<div data-tauri-drag-region class="ml-2 flex min-w-0 grow items-center gap-2">
		<SearchIcon data-tauri-drag-region class="size-3.5 shrink-0 opacity-30" />
		<input
			bind:value={query}
			placeholder="Find a note"
			class="min-w-0 grow bg-transparent text-[12.5px] outline-none placeholder:opacity-30"
		/>
	</div>
{/snippet}

<div class="flex h-full flex-col">
	<!-- Filters, above the grid and below the bar: they describe what is on
	     screen, so they sit with it rather than in the window chrome. -->
	<div class="flex shrink-0 flex-wrap items-center gap-1.5 px-6 pt-4 pb-3">
		{#each SCOPES as s (s.id)}
			<button
				type="button"
				onclick={() => (scope = s.id)}
				title={s.hint}
				class="rounded-md border px-2.5 py-1 text-[11.5px] transition-colors
					{scope === s.id
					? 'bg-foreground text-background border-transparent'
					: 'hover:bg-accent'}"
				style={scope === s.id ? '' : 'border-color: var(--hairline)'}
			>
				{s.label}
			</button>
		{/each}

		{#if tag}
			<!-- Removable, and shown as itself: a filter you can see is a filter
			     you can get out of. -->
			<button
				type="button"
				onclick={clearTag}
				title="Clear this tag"
				class="bg-accent hover:bg-accent/70 flex items-center gap-1 rounded-md px-2.5 py-1 text-[11.5px] transition-colors"
			>
				<span class="font-mono opacity-35">#</span>
				<span class="max-w-[180px] truncate">{tag.name}</span>
				<XIcon class="size-3 opacity-45" />
			</button>
		{/if}

		<span class="ml-auto font-mono text-[10px] tabular-nums opacity-30">
			{notes.length}
		</span>
	</div>

	<div class="min-h-0 grow overflow-y-auto px-6 pb-8">
		{#if notes.length === 0}
			<p class="text-muted-foreground py-10 text-[13px] leading-relaxed">
				{query || tag || scope !== 'all'
					? 'Nothing matches.'
					: 'No notes yet. Capture something with Ctrl N.'}
			</p>
		{:else}
			<div class="grid grid-cols-[repeat(auto-fill,minmax(210px,1fr))] gap-3">
				{#each notes as note (note.id)}
					<button
						type="button"
						onclick={() => (editingNoteId = note.id)}
						onpointerdown={(e) => grab(e, note)}
						oncontextmenu={(e) => cardMenu(e, note)}
						title="Click to open, drag onto a board to place it"
						class="bg-card hover:border-foreground/25 flex max-h-64 min-h-24 cursor-grab
							touch-none flex-col gap-2 overflow-hidden rounded-lg border p-4 text-left
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
							<span class="min-w-0 text-[13.5px] leading-snug font-semibold">
								{cardFace(note) || 'Untitled'}
							</span>
						</div>

						{#if excerpt(note)}
							<p class="text-muted-foreground min-h-0 overflow-hidden text-[12px] leading-relaxed">
								{excerpt(note)}
							</p>
						{/if}
					</button>
				{/each}
			</div>
		{/if}
	</div>
</div>

{#if editingNoteId}
	{#key editingNoteId}
		<NoteEditor noteId={editingNoteId} onclose={() => (editingNoteId = null)} />
	{/key}
{/if}
