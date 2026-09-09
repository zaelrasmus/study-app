<script lang="ts">
	/**
	 * Document order, made visible and reorderable.
	 *
	 * This strip is the *only* place order changes. Dragging a frame around the
	 * canvas is thinking and must never renumber anything; dragging a chip here
	 * is editing. Keeping those two gestures apart is what lets the canvas stay
	 * free — and having the order permanently on screen is what stops the canvas
	 * and the document quietly disagreeing.
	 *
	 * Reordering uses pointer events, not HTML5 drag-and-drop: Tauri's webview
	 * claims the native drag handler for file drops, so `dragstart` fires and
	 * nothing after it ever does.
	 */
	import { onMount } from 'svelte';
	import { drag, type DragPayload, type DropContext } from '$lib/state/drag.svelte';
	import type { Frame } from '$lib/types';
	import GripIcon from '@lucide/svelte/icons/grip-vertical';

	let {
		frames,
		counts,
		onreorder,
		onfocus
	}: {
		frames: Frame[];
		counts: Record<string, number>;
		onreorder: (ids: string[]) => void;
		onfocus?: (frameId: string) => void;
	} = $props();

	const ordered = $derived([...frames].sort((a, b) => a.order_index - b.order_index));

	function drop(payload: DragPayload, ctx: DropContext) {
		if (payload.kind !== 'frame') return;

		const targetId = ctx.target.dataset.frameId;
		if (!targetId || targetId === payload.frameId) return;

		const ids = ordered.map((f) => f.id);
		const from = ids.indexOf(payload.frameId);
		const to = ids.indexOf(targetId);
		if (from < 0 || to < 0) return;

		ids.splice(to, 0, ids.splice(from, 1)[0]);
		onreorder(ids);
	}

	onMount(() => drag.register('frame-chip', drop));
</script>

{#if ordered.length > 0}
	<div
		class="absolute top-14 left-3 z-20 flex max-w-[calc(100%-6rem)] items-center gap-1 overflow-x-auto"
	>
		<span
			class="label bg-card/95 shrink-0 rounded border px-1.5 py-1 backdrop-blur"
			style="border-color: var(--hairline)"
		>
			doc
		</span>

		{#each ordered as frame, i (frame.id)}
			{@const dragging = drag.payload?.kind === 'frame' && drag.payload.frameId === frame.id}
			<div
				role="listitem"
				data-drop-zone="frame-chip"
				data-frame-id={frame.id}
				onpointerdown={(e) =>
					drag.begin({ kind: 'frame', frameId: frame.id, label: frame.label || 'untitled' }, e)}
				class="bg-card/95 flex shrink-0 cursor-grab touch-none items-center gap-1.5 rounded border py-1 pr-2 pl-1 shadow-sm backdrop-blur transition-all active:cursor-grabbing
					{dragging ? 'opacity-40' : ''}"
				style="border-color: {drag.over === 'frame-chip' && !dragging && drag.payload
					? 'var(--foreground)'
					: 'var(--hairline)'}"
			>
				<GripIcon class="size-3 opacity-25" />

				<span
					class="bg-foreground text-background flex h-[15px] min-w-[15px] items-center justify-center rounded-[2px] px-1 font-mono text-[9px] tabular-nums"
				>
					{i + 1}
				</span>

				<button
					type="button"
					onclick={() => onfocus?.(frame.id)}
					title="Find this frame on the canvas"
					class="max-w-40 truncate text-[11px] {frame.label ? '' : 'opacity-35 italic'}"
				>
					{frame.label || 'untitled'}
				</button>

				{#if counts[frame.id]}
					<span class="font-mono text-[9px] opacity-35 tabular-nums">{counts[frame.id]}</span>
				{/if}
			</div>
		{/each}
	</div>
{/if}
