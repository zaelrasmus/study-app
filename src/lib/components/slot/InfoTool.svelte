<script lang="ts">
	/**
	 * What the app knows about this note.
	 *
	 * Facts only, laid out as a definition list. The recall history appears here
	 * and nowhere else in this much detail — and only when the note has one. A
	 * note that never attempted memory shows the boards and the timestamps, and
	 * says nothing about a pipeline it did not enter.
	 *
	 * Backlinks belong here too, but mentions are not built yet, so the section
	 * is honestly absent rather than present and permanently empty.
	 */
	import * as ipc from '$lib/ipc';
	import { data } from '$lib/state/data.svelte';
	import type { Note, Topic } from '$lib/types';
	import { cardFace } from '$lib/types';
	import { STATE_LABEL, STATE_MEANING } from '$lib/types';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import { goto } from '$app/navigation';

	let {
		noteId,
		onopen
	}: {
		noteId: string;
		/** Following a backlink opens that note. Absent where nothing can open one. */
		onopen?: (noteId: string) => void;
	} = $props();

	let note = $state<Note | null>(null);
	let boards = $state<Topic[]>([]);
	let tags = $state<ipc.Tag[]>([]);
	let backlinks = $state<Note[]>([]);
	let adding = $state(false);
	let draft = $state('');

	$effect(() => {
		const id = noteId;
		ipc.getNote(id).then((n) => (note = n));
		data.notes;
		data.tags;
		data.boards;
		ipc.noteBoards(id).then((b) => (boards = b));
		ipc.noteTags(id).then((t) => (tags = t));
		ipc.noteBacklinks(id).then((b) => (backlinks = b));
	});

	function when(iso: string | null) {
		if (!iso) return null;
		const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
		if (days < 1) return 'today';
		if (days === 1) return 'yesterday';
		if (days < 30) return `${days}d ago`;
		if (days < 365) return `${Math.floor(days / 30)}mo ago`;
		return `${Math.floor(days / 365)}y ago`;
	}

	async function addTag() {
		const name = draft.trim();
		draft = '';
		adding = false;
		if (!name) return;

		await ipc.attachTag(noteId, name);
		tags = await ipc.noteTags(noteId);
	}

	async function removeTag(tagId: string) {
		await ipc.detachTag(noteId, tagId);
		tags = await ipc.noteTags(noteId);
	}

	const words = $derived(
		note ? note.body_text.trim().split(/\s+/).filter(Boolean).length : 0
	);
</script>

{#if note}
	<div class="min-h-0 overflow-y-auto px-4 py-3">
		<dl class="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-2 text-[11px]">
			<dt class="label !text-[9px]">Created</dt>
			<dd>{when(note.created_at)}</dd>

			<dt class="label !text-[9px]">Updated</dt>
			<dd>{when(note.updated_at)}</dd>

			<dt class="label !text-[9px]">Length</dt>
			<dd class="tabular-nums">{words} words</dd>
		</dl>

		<!-- Boards, plural and deliberately so: a note can sit on several at once,
		     because boards import notes rather than owning them. -->
		<div class="mt-4 border-t pt-3" style="border-color: var(--hairline)">
			<p class="label mb-1.5">On boards</p>
			{#each boards as board (board.id)}
				<button
					type="button"
					onclick={() => goto(`/t/${board.id}`)}
					class="hover:bg-accent flex w-full items-center gap-2 rounded px-1.5 py-1 text-left text-[11.5px] transition-colors"
				>
					<span class="grow truncate">{board.title || 'Untitled'}</span>
					{#if board.study}
						<span class="label !text-[8px]">study</span>
					{/if}
				</button>
			{:else}
				<p class="text-muted-foreground px-1.5 text-[11px] leading-relaxed">
					Not on a board. That is a normal place for a note to be — drag it onto one
					from the library when it belongs somewhere.
				</p>
			{/each}
		</div>

		<div class="mt-4 border-t pt-3" style="border-color: var(--hairline)">
			<div class="mb-1.5 flex items-center gap-2">
				<p class="label grow">Tags</p>
				<button
					type="button"
					onclick={() => (adding = true)}
					class="hover:bg-accent rounded px-1 font-mono text-[9px] uppercase opacity-45 transition-all hover:opacity-100"
				>
					add
				</button>
			</div>

			<div class="flex flex-wrap gap-1">
				{#each tags as tag (tag.id)}
					<button
						type="button"
						onclick={() => removeTag(tag.id)}
						title="Remove"
						class="hover:border-foreground rounded border px-1.5 py-0.5 text-[10.5px] transition-colors"
						style="border-color: var(--hairline-strong)"
					>
						{tag.name}
					</button>
				{/each}

				{#if adding}
					<!-- svelte-ignore a11y_autofocus -->
					<input
						autofocus
						bind:value={draft}
						placeholder="tag"
						onblur={addTag}
						onkeydown={(e) => {
							if (e.key === 'Enter') addTag();
							if (e.key === 'Escape') {
								adding = false;
								draft = '';
							}
						}}
						class="bg-background w-20 rounded border px-1.5 py-0.5 text-[10.5px] outline-none"
						style="border-color: var(--hairline)"
					/>
				{:else if tags.length === 0}
					<p class="text-muted-foreground text-[11px]">None.</p>
				{/if}
			</div>
		</div>

		<!-- Mentions are always deliberate, so backlinks are only ever notes you
		     chose to point here. Nothing was inferred and nothing was suggested. -->
		{#if backlinks.length > 0}
			<div class="mt-4 border-t pt-3" style="border-color: var(--hairline)">
				<p class="label mb-1.5">Mentioned by {backlinks.length}</p>
				{#each backlinks as source (source.id)}
					<button
						type="button"
						onclick={() => onopen?.(source.id)}
						class="hover:bg-accent block w-full truncate rounded px-1.5 py-1 text-left text-[11.5px] transition-colors"
					>
						{cardFace(source) || 'Untitled'}
					</button>
				{/each}
			</div>
		{/if}

		<!-- The memory axis, and only when there is one. -->
		{#if note.visible}
			<div class="mt-4 border-t pt-3" style="border-color: var(--hairline)">
				<p class="label mb-1.5">Memory</p>

				<div class="mb-2 flex items-center gap-2 text-[11.5px]">
					<StateMark
						state={note.state}
						visible={note.visible}
						neverContrasted={note.never_contrasted}
					/>
					<span>{STATE_LABEL[note.state]}</span>
				</div>

				<p class="text-muted-foreground mb-2.5 text-[11px] leading-relaxed">
					{STATE_MEANING[note.state]}
				</p>

				<dl class="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-2 text-[11px]">
					<dt class="label !text-[9px]">Reproduced</dt>
					<dd>{when(note.last_recall_at)}</dd>

					<dt class="label !text-[9px]">Contrasted</dt>
					<dd>
						{#if note.last_checked_at}
							{when(note.last_checked_at)}
						{:else}
							<span class="opacity-50">never</span>
						{/if}
					</dd>

					{#if note.last_recall_failed_at}
						<dt class="label !text-[9px]">Last missed</dt>
						<dd>{when(note.last_recall_failed_at)}</dd>
					{/if}
				</dl>
			</div>
		{/if}
	</div>
{/if}
