<script lang="ts">
	/**
	 * The same slot, contextual content: frames on a canvas, headings in a note.
	 *
	 * On a work board the frames are listed without numbers — there is no
	 * document, so there is no position in one to announce.
	 */
	import type { Frame } from '$lib/types';

	let {
		frames = [],
		study = false,
		onfocus
	}: { frames?: Frame[]; study?: boolean; onfocus?: (frameId: string) => void } = $props();

	const ordered = $derived([...frames].sort((a, b) => a.order_index - b.order_index));
</script>

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
