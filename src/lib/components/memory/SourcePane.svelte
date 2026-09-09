<script lang="ts">
	/**
	 * The source: whatever the app is holding that you could copy from.
	 *
	 * Two things, and only two, per the model:
	 *   - the long note being distilled, when writing an atomic note from it
	 *   - the previous version of this note, when re-recalling it
	 *
	 * External sources — PDFs, URLs, highlights — are not here yet, and when they
	 * arrive memory mode will make no claim about them. The app hides what the
	 * app holds. It cannot hide your second monitor and does not pretend to,
	 * which is why the footer says so out loud rather than implying a guarantee
	 * it can't keep.
	 *
	 * This component is only ever rendered when memory mode is off or during
	 * Contrast. In memory mode the parent removes it from the DOM entirely: if it
	 * can be looked at, it will be.
	 */
	import * as ipc from '$lib/ipc';
	import type { Note } from '$lib/types';
	import '$lib/components/edra/shadcn/editor.css';

	let {
		sourceNoteId = null,
		outgoing = null,
		onpick
	}: {
		sourceNoteId?: string | null;
		/** The body this note had before the blind rewrite, held for Contrast. */
		outgoing?: { title: string; html: string; text: string } | null;
		onpick?: () => void;
	} = $props();

	let sourceNote = $state<Note | null>(null);

	$effect(() => {
		const id = sourceNoteId;
		if (!id) {
			sourceNote = null;
			return;
		}
		ipc.getNote(id).then((n) => (sourceNote = n));
	});

	/** The previous version wins: when re-recalling, that is what you were
	 *  actually reproducing. */
	const showing = $derived(
		outgoing
			? { kind: 'previous' as const, title: outgoing.title, html: outgoing.html }
			: sourceNote
				? { kind: 'note' as const, title: sourceNote.title, html: sourceNote.body_html }
				: null
	);
</script>

<aside
	class="bg-muted/25 flex h-full min-h-0 w-full flex-col border-r"
	style="border-color: var(--hairline)"
>
	<header
		class="flex shrink-0 items-center gap-2 border-b px-5 py-2.5"
		style="border-color: var(--hairline)"
	>
		<span class="label grow">
			{showing?.kind === 'previous' ? 'Previous version' : 'Source'}
		</span>
		{#if showing?.kind !== 'previous' && onpick}
			<button
				type="button"
				onclick={onpick}
				class="hover:bg-accent rounded px-1.5 py-0.5 font-mono text-[9px] tracking-wider uppercase opacity-50 transition-all hover:opacity-100"
			>
				{sourceNoteId ? 'change' : 'choose'}
			</button>
		{/if}
	</header>

	<div class="selectable min-h-0 grow overflow-y-auto px-5 py-4">
		{#if showing}
			{#if showing.title}
				<h3 class="mb-2 font-serif text-[17px] font-semibold tracking-tight">
					{showing.title}
				</h3>
			{/if}
			{#if showing.html.trim()}
				<div class="tiptap prose-doc">{@html showing.html}</div>
			{:else}
				<p class="text-muted-foreground text-[12px] italic">This note has no body.</p>
			{/if}
		{:else}
			<p class="text-muted-foreground text-[12px] leading-relaxed">
				No source. Choose the long note you are distilling from, or just write — a note
				with no source is still a note.
			</p>
		{/if}
	</div>

	<footer class="shrink-0 border-t px-5 py-2" style="border-color: var(--hairline)">
		<p class="text-muted-foreground text-[10px] leading-relaxed">
			Memory mode hides what the app holds. It cannot hide anything open elsewhere.
		</p>
	</footer>
</aside>
