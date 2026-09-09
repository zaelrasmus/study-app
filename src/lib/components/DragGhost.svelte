<script lang="ts">
	/**
	 * What follows the cursor during a drag.
	 *
	 * `pointer-events: none` is load-bearing: hit-testing on release uses
	 * `elementFromPoint`, and a ghost that intercepted the cursor would report
	 * itself as the drop target every time.
	 */
	import { drag } from '$lib/state/drag.svelte';

	/** A question is open rather than had, and the ghost says so while it flies. */
	const question = $derived(drag.payload?.kind === 'question');
</script>

{#if drag.payload}
	<div
		class="bg-card pointer-events-none fixed z-[100] max-w-64 truncate rounded px-2.5 py-1.5 shadow-lg
			{question ? 'font-serif text-[12px]' : 'text-[11.5px]'}"
		style="left: {drag.x + 12}px; top: {drag.y + 12}px;
			border: 1px {question ? 'dashed' : 'solid'} var(--hairline-strong)"
	>
		{drag.payload.label || 'Untitled'}
	</div>
{/if}
