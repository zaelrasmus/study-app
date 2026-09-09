<script lang="ts">
	/**
	 * A frame: a labelled region, and the unit of completion.
	 *
	 * Two details carry the design. The order badge is explicit and visible, so
	 * the canvas and the document can never quietly disagree — dragging a frame
	 * around to think renumbers nothing. And the frame is only draggable by its
	 * tab (`dragHandle: '.frame-handle'` plus `pointer-events-none` on the body),
	 * so grabbing a note that sits inside a frame never grabs the frame instead.
	 */
	import { NodeResizer, type NodeProps } from '@xyflow/svelte';
	import type { Frame } from '$lib/types';
	import XIcon from '@lucide/svelte/icons/x';

	type Data = {
		frame: Frame;
		study: boolean;
		count: number;
		onlabel: (id: string, label: string) => void;
		ondelete: (id: string) => void;
	};

	let { data, selected }: NodeProps = $props();

	const frame = $derived((data as Data).frame);
	const count = $derived((data as Data).count);
	/** No document on a work board, so no position in one. */
	const study = $derived((data as Data).study);

	let editing = $state(false);
	let draft = $state('');

	function start() {
		draft = frame.label;
		editing = true;
	}

	function commit() {
		editing = false;
		if (draft.trim() !== frame.label) (data as Data).onlabel(frame.id, draft.trim());
	}
</script>

<NodeResizer
	isVisible={selected}
	minWidth={280}
	minHeight={200}
	lineClass="!border-transparent"
	handleClass="!size-2 !rounded-none !bg-background"
/>

<!-- The region itself: a hairline, nothing more. It must never look like a card,
     or it competes with the notes inside it. -->
<div
	class="h-full w-full rounded-xl border transition-colors"
	style="border-color: {selected ? 'var(--foreground)' : 'var(--frame-line)'};
		border-style: dashed;
		background: color-mix(in oklch, var(--foreground) {selected ? 3 : 1.5}%, transparent)"
></div>

<!-- The tab. The only part with pointer events, and therefore the only handle. -->
<header class="frame-handle absolute -top-[26px] left-0 flex cursor-grab items-center gap-2 active:cursor-grabbing">
	{#if study}
		<span
			class="bg-foreground text-background flex h-[18px] min-w-[18px] items-center justify-center rounded-[3px] px-1 font-mono text-[10px] tabular-nums"
			title="Position in the document"
		>
			{frame.order_index + 1}
		</span>
	{/if}

	{#if editing}
		<!-- svelte-ignore a11y_autofocus -->
		<input
			autofocus
			bind:value={draft}
			onblur={commit}
			onkeydown={(e) => {
				if (e.key === 'Enter') commit();
				if (e.key === 'Escape') editing = false;
			}}
			placeholder="Section name"
			class="bg-background w-44 rounded border px-1.5 py-0.5 text-[11px] outline-none"
		/>
	{:else}
		<button
			type="button"
			ondblclick={start}
			class="text-[11px] font-medium tracking-tight transition-opacity hover:opacity-100 {frame.label
				? 'opacity-70'
				: 'opacity-35 italic'}"
		>
			{frame.label || 'untitled section'}
		</button>
	{/if}

	<span class="font-mono text-[9px] opacity-30 tabular-nums">
		{count || ''}
	</span>

	{#if selected}
		<button
			type="button"
			onclick={() => (data as Data).ondelete(frame.id)}
			title="Remove the frame (the notes stay)"
			class="hover:bg-accent flex size-4 items-center justify-center rounded opacity-40 transition-all hover:opacity-100"
		>
			<XIcon class="size-2.5" />
		</button>
	{/if}
</header>
