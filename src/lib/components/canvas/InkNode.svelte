<script lang="ts">
	/**
	 * A freehand stroke.
	 *
	 * Strokes keep their input points rather than a rasterised image, so they
	 * re-render crisply at any zoom and can be restyled later without loss.
	 */
	import { type NodeProps } from '@xyflow/svelte';
	import { strokePath } from '$lib/ink';
	import type { Ink } from '$lib/types';

	type Data = { ink: Ink; oncontext: (event: MouseEvent) => void };

	let { data, selected }: NodeProps = $props();

	const ink = $derived((data as Data).ink);
	const path = $derived(strokePath(ink));

	const bounds = $derived.by(() => {
		const xs = ink.points.map((p) => p[0]);
		const ys = ink.points.map((p) => p[1]);
		const pad = ink.size;
		return {
			minX: Math.min(...xs) - pad,
			minY: Math.min(...ys) - pad,
			width: Math.max(...xs) - Math.min(...xs) + pad * 2,
			height: Math.max(...ys) - Math.min(...ys) + pad * 2
		};
	});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<svg
	class="overflow-visible {selected ? 'opacity-70' : ''}"
	width={bounds.width}
	height={bounds.height}
	viewBox="{bounds.minX} {bounds.minY} {bounds.width} {bounds.height}"
	oncontextmenu={(e) => (data as Data).oncontext(e)}
	role="presentation"
>
	<path d={path} class="fill-ink" />
</svg>
