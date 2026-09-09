<script lang="ts">
	/**
	 * Drag out a rectangle to make a frame.
	 *
	 * Drawing the region is far more legible than a button that drops a fixed box
	 * somewhere and leaves you to find it: you say where the section is, and the
	 * notes already inside it join it on release.
	 */
	import { useSvelteFlow } from '@xyflow/svelte';

	let {
		oncommit,
		onescape
	}: {
		oncommit: (rect: { x: number; y: number; width: number; height: number }) => void;
		onescape: () => void;
	} = $props();

	const flow = useSvelteFlow();

	let start = $state<{ x: number; y: number } | null>(null);
	let current = $state<{ x: number; y: number } | null>(null);

	/** Screen-space rect, for drawing the marquee. */
	const rect = $derived.by(() => {
		if (!start || !current) return null;
		return {
			left: Math.min(start.x, current.x),
			top: Math.min(start.y, current.y),
			width: Math.abs(current.x - start.x),
			height: Math.abs(current.y - start.y)
		};
	});

	function onpointerdown(event: PointerEvent) {
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		start = { x: event.clientX, y: event.clientY };
		current = start;
	}

	function onpointermove(event: PointerEvent) {
		if (!start) return;
		current = { x: event.clientX, y: event.clientY };
	}

	function onpointerup() {
		if (rect && rect.width > 24 && rect.height > 24) {
			// Convert both corners so the frame lands exactly where it was drawn,
			// at any zoom level.
			const a = flow.screenToFlowPosition({ x: rect.left, y: rect.top });
			const b = flow.screenToFlowPosition({
				x: rect.left + rect.width,
				y: rect.top + rect.height
			});

			oncommit({ x: a.x, y: a.y, width: b.x - a.x, height: b.y - a.y });
		}

		start = null;
		current = null;
	}
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onescape()} />

<div
	class="absolute inset-0 z-10 cursor-crosshair touch-none"
	role="presentation"
	{onpointerdown}
	{onpointermove}
	{onpointerup}
	onpointercancel={onpointerup}
>
	{#if rect}
		<!-- `fixed`, because the coordinates are viewport-space (clientX/clientY)
		     while this overlay sits inside a container offset by the sidebar and
		     the titlebar. -->
		<div
			class="pointer-events-none fixed rounded-xl border border-dashed"
			style="left:{rect.left}px; top:{rect.top}px; width:{rect.width}px; height:{rect.height}px;
				border-color: var(--foreground); background: color-mix(in oklch, var(--foreground) 4%, transparent)"
		></div>
	{/if}
</div>
