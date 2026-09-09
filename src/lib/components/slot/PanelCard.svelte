<script lang="ts">
	/**
	 * One card in a panel.
	 *
	 * The panels used to be bare rows of text, which read as a dump rather than
	 * as things — and they looked nothing like the library, where the same notes
	 * are cards. This is the library's card at panel width: one column, less
	 * padding, same edge and same radius, so the whole app agrees about what a
	 * thing looks like.
	 */
	import type { Snippet } from 'svelte';

	let {
		children,
		onclick,
		ondblclick,
		oncontextmenu,
		onpointerdown,
		title,
		grab = false,
		dashed = false
	}: {
		children: Snippet;
		onclick?: (event: MouseEvent) => void;
		ondblclick?: (event: MouseEvent) => void;
		oncontextmenu?: (event: MouseEvent) => void;
		onpointerdown?: (event: PointerEvent) => void;
		title?: string;
		/** Draggable onto a board, so the cursor should say so. */
		grab?: boolean;
		/** For things that are open rather than had: questions, mostly. */
		dashed?: boolean;
	} = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	role={onclick || ondblclick ? 'button' : undefined}
	tabindex={onclick || ondblclick ? 0 : undefined}
	{title}
	{onclick}
	{ondblclick}
	{oncontextmenu}
	{onpointerdown}
	onkeydown={(e) => {
		if (e.key !== 'Enter' && e.key !== ' ') return;
		const act = onclick ?? ondblclick;
		if (!act) return;
		e.preventDefault();
		act(e as unknown as MouseEvent);
	}}
	class="bg-card hover:border-foreground/25 mb-1.5 flex flex-col gap-1.5 rounded-md p-2.5
		text-left transition-colors {grab ? 'cursor-grab touch-none active:cursor-grabbing' : ''}"
	style="border: 1px {dashed ? 'dashed' : 'solid'} var(--hairline)"
>
	{@render children()}
</div>
