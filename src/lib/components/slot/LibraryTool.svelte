<script lang="ts">
	/**
	 * The card list beside a board.
	 *
	 * This is the narrow survivor of the library: on a canvas its only job is to
	 * be a drag source, so you can pull an existing note onto the board instead
	 * of retyping it. Everything else about the corpus — the grid, tags, the
	 * count — lives on the Library screen, where there is room to read it.
	 *
	 * Boards do not own notes, they import them. Being on no canvas is the
	 * normal state of a note, so "unplaced" is a filter here, not a condition to
	 * be fixed, and it carries no count.
	 */
	import * as ipc from '$lib/ipc';
	import type { Note } from '$lib/types';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import PanelCard from './PanelCard.svelte';
	import SearchIcon from '@lucide/svelte/icons/search';
	import { drag } from '$lib/state/drag.svelte';
	import { data } from '$lib/state/data.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import { cardFace } from '$lib/types';

	let { topicId = null }: { topicId?: string | null } = $props();

	type Scope = 'all' | 'board' | 'inbox';

	let scope = $state<Scope>('all');
	let query = $state('');
	let notes = $state<Note[]>([]);

	const scopes: { id: Scope; label: string }[] = $derived([
		{ id: 'all', label: 'All' },
		...(topicId ? [{ id: 'board' as const, label: 'On this board' }] : []),
		{ id: 'inbox', label: 'Unplaced' }
	]);

	$effect(() => {
		// Re-reads when a note changes anywhere, or when a board's contents do.
		data.notes;
		data.boards;
		const s = scope;
		const q = query;
		ipc
			.libraryNotes({
				query: q || undefined,
				topicId: s === 'board' ? topicId : null,
				unplacedOnly: s === 'inbox'
			})
			.then((found) => (notes = found));
	});

	/**
	 * Dragging a note onto the canvas is how a board gets composed.
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
	 * Deleting from the panel is deleting everywhere — a note lives in the
	 * library, not on the board it happens to be shown beside — so it asks.
	 */
	function cardMenu(event: MouseEvent, note: Note) {
		menu.show(event, [
			{ label: 'Open', onpick: () => slot.openNote?.(note.id) },
			{
				label: 'Delete note',
				destructive: true,
				onpick: async () => {
					const face = cardFace(note) || 'Untitled';
					if (!confirm(`Delete "${face}"? This cannot be undone.`)) return;
					notes = notes.filter((n) => n.id !== note.id);
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
		return rest.replace(/\s+/g, ' ').trim().slice(0, 140);
	}
</script>

<div class="flex min-h-0 flex-col">
	<div class="shrink-0 px-3 pt-2.5 pb-2">
		<div
			class="bg-background flex h-8 items-center gap-2 rounded-md border px-2.5"
			style="border-color: var(--hairline)"
		>
			<SearchIcon class="size-3.5 shrink-0 opacity-30" />
			<input
				bind:value={query}
				placeholder="Search cards"
				class="w-full bg-transparent text-[12px] outline-none placeholder:opacity-30"
			/>
		</div>

		<div class="mt-2 flex flex-wrap gap-1">
			{#each scopes as s (s.id)}
				<button
					type="button"
					onclick={() => (scope = s.id)}
					class="rounded px-2 py-0.5 text-[11px] transition-colors
						{scope === s.id ? 'bg-foreground text-background' : 'hover:bg-accent opacity-45'}"
				>
					{s.label}
				</button>
			{/each}
		</div>
	</div>

	<div class="min-h-0 grow overflow-y-auto px-2 pb-2">
		{#each notes as note (note.id)}
			<PanelCard
				grab
				onpointerdown={(e) => grab(e, note)}
				ondblclick={() => slot.openNote?.(note.id)}
				oncontextmenu={(e) => cardMenu(e, note)}
				title="Double-click to open · drag onto the canvas to place it"
			>
				<div class="flex items-start gap-2">
					<StateMark
						state={note.state}
						visible={note.visible}
						neverContrasted={note.never_contrasted}
						class="mt-[4px]"
					/>
					<span class="min-w-0 grow text-[12px] leading-snug font-medium">
						{cardFace(note) || 'Untitled'}
					</span>
				</div>
				{#if excerpt(note)}
					<p class="text-muted-foreground line-clamp-2 text-[11px] leading-relaxed">
						{excerpt(note)}
					</p>
				{/if}
			</PanelCard>
		{:else}
			<p class="text-muted-foreground px-2 py-3 text-[12px] leading-relaxed">
				{query ? 'Nothing matches.' : 'No notes yet. Capture something with Ctrl N.'}
			</p>
		{/each}
	</div>
</div>
