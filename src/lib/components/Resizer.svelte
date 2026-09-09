<script lang="ts">
	/**
	 * The seam between a panel and the working area.
	 *
	 * Pointer events, like everything else that drags here — and a 5px hit area
	 * over a 1px line, because a seam you have to aim at is a seam you avoid.
	 */
	let {
		side,
		onresize
	}: { side: 'left' | 'right'; onresize: (px: number) => void } = $props();

	let dragging = $state(false);

	function down(event: PointerEvent) {
		if (event.button !== 0) return;
		dragging = true;

		const move = (e: PointerEvent) => {
			// Measured from the window edge, so the panel follows the cursor
			// exactly rather than drifting by whatever offset it started at.
			onresize(side === 'left' ? e.clientX : window.innerWidth - e.clientX);
		};

		const up = () => {
			dragging = false;
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
		};

		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}
</script>

<div
	role="separator"
	aria-orientation="vertical"
	onpointerdown={down}
	class="relative z-30 w-px shrink-0 cursor-col-resize touch-none"
	style="background: {dragging ? 'var(--foreground)' : 'var(--hairline)'}"
>
	<!-- The real hit area, wider than the line it controls. -->
	<div class="absolute inset-y-0 -left-[3px] w-[7px]"></div>
</div>
