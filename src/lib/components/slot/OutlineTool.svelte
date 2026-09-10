<script lang="ts">
	/**
	 * The same slot, contextual content: frames on a canvas, headings in a
	 * document.
	 *
	 * On a work board the frames are listed without numbers — there is no
	 * document, so there is no position in one to announce.
	 *
	 * The headings half is the table of contents this panel has always claimed
	 * to have and never had. It reads the live document, so a heading appears
	 * here as you finish typing it, and clicking one puts the caret in it rather
	 * than merely scrolling to it: you came to the panel to go somewhere and
	 * carry on writing.
	 *
	 * Indentation is the only thing levels are used for. Nothing is collapsed
	 * and nothing is hidden below a depth — a messy page with eight H3s and no
	 * H1 is still a shape worth seeing, and it is the shape you actually write.
	 */
	import type { Frame } from '$lib/types';
	import type { Heading } from '$lib/outline';

	let {
		frames = [],
		headings = [],
		document: isDocument = false,
		study = false,
		onfocus,
		onfocusheading
	}: {
		frames?: Frame[];
		headings?: Heading[];
		/** True when the centre is a document, so headings are the right answer. */
		document?: boolean;
		study?: boolean;
		onfocus?: (frameId: string) => void;
		onfocusheading?: (pos: number) => void;
	} = $props();

	const ordered = $derived([...frames].sort((a, b) => a.order_index - b.order_index));

	/**
	 * The shallowest heading present.
	 *
	 * Indenting from H1 would push a document written entirely in H2s a level
	 * off the wall for no reason. What matters is the relative shape.
	 */
	const base = $derived(headings.length ? Math.min(...headings.map((h) => h.level)) : 1);
</script>

{#if isDocument}
	<div class="min-h-0 overflow-y-auto px-3 py-2">
		{#each headings as heading (heading.id)}
			<button
				type="button"
				onclick={() => onfocusheading?.(heading.pos)}
				title="Go to this heading"
				class="hover:bg-accent flex w-full items-baseline gap-2 rounded py-1.5 pr-2 text-left transition-colors"
				style="padding-left: {8 + Math.min(heading.level - base, 3) * 12}px"
			>
				<span
					class="grow truncate {heading.level === base
						? 'text-[12px] font-medium'
						: 'text-[11.5px] opacity-80'} {heading.text ? '' : 'italic opacity-40'}"
				>
					{heading.text || 'untitled heading'}
				</span>
			</button>
		{:else}
			<p class="text-muted-foreground px-2 py-3 text-[11px] leading-relaxed">
				No headings yet. Mark a line as a heading and it turns up here —
				clicking it puts the caret back in that spot.
			</p>
		{/each}
	</div>
{:else}
	<div class="min-h-0 overflow-y-auto px-3 py-2">
		{#each ordered as frame, i (frame.id)}
			<button
				type="button"
				onclick={() => onfocus?.(frame.id)}
				class="hover:bg-accent flex w-full items-baseline gap-2 rounded px-2 py-1.5 text-left transition-colors"
			>
				{#if study}
					<span class="font-mono text-[9px] tabular-nums opacity-35">
						{String(i + 1).padStart(2, '0')}
					</span>
				{/if}
				<span class="grow truncate text-[11.5px] {frame.label ? '' : 'opacity-40 italic'}">
					{frame.label || 'untitled section'}
				</span>
			</button>
		{:else}
			<p class="text-muted-foreground px-2 py-3 text-[11px] leading-relaxed">
				No frames yet. Press F and drag one around a cluster of notes.
			</p>
		{/each}
	</div>
{/if}
