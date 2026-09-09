<script lang="ts">
	/**
	 * The drawing layer.
	 *
	 * Sits above the flow while the pen is active and captures pointer input in
	 * flow coordinates, so a stroke lands where it was drawn regardless of pan
	 * or zoom. The live stroke is rendered in screen space for zero latency and
	 * is only converted on release.
	 */
	import { useSvelteFlow } from '@xyflow/svelte';
	import getStroke from 'perfect-freehand';
	import { DEFAULT_INK } from '$lib/ink';

	let {
		oncommit,
		onescape
	}: {
		oncommit: (points: [number, number, number][]) => void;
		onescape: () => void;
	} = $props();

	const flow = useSvelteFlow();

	/** Screen-space points, for drawing the stroke as it happens. */
	let live = $state<[number, number, number][]>([]);
	/** Flow-space points, which are what gets stored. */
	let captured: [number, number, number][] = [];

	const preview = $derived.by(() => {
		if (live.length === 0) return '';

		const outline = getStroke(live, {
			size: DEFAULT_INK.size,
			thinning: DEFAULT_INK.thinning,
			smoothing: DEFAULT_INK.smoothing,
			streamline: DEFAULT_INK.streamline,
			simulatePressure: true
		});

		if (outline.length === 0) return '';

		const d = outline.reduce<string[]>((acc, [x0, y0], i, arr) => {
			const [x1, y1] = arr[(i + 1) % arr.length];
			acc.push(`${x0.toFixed(2)},${y0.toFixed(2)}`, `${((x0 + x1) / 2).toFixed(2)},${((y0 + y1) / 2).toFixed(2)}`);
			return acc;
		}, []);

		return `M${d[0]} Q${d.slice(1).join(' ')} Z`;
	});

	function record(event: PointerEvent) {
		live.push([event.clientX, event.clientY, event.pressure || 0.5]);
		live = live;

		const flowPoint = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
		captured.push([flowPoint.x, flowPoint.y, event.pressure || 0.5]);
	}

	function onpointerdown(event: PointerEvent) {
		(event.target as HTMLElement).setPointerCapture(event.pointerId);
		live = [];
		captured = [];
		record(event);
	}

	function onpointermove(event: PointerEvent) {
		if (live.length === 0) return;
		// Coalesced events keep fast strokes smooth on high-rate pointers.
		for (const e of event.getCoalescedEvents?.() ?? [event]) record(e);
	}

	function onpointerup() {
		// A tap is not a stroke.
		if (captured.length > 1) oncommit(captured);
		live = [];
		captured = [];
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
	{#if preview}
		<!-- `fixed`, because the live points are viewport-space (clientX/clientY)
		     while this overlay sits inside a container offset by the sidebar and
		     the titlebar. -->
		<svg class="pointer-events-none fixed inset-0 h-screen w-screen">
			<path d={preview} class="fill-ink" />
		</svg>
	{/if}
</div>
