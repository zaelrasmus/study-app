<script lang="ts">
	/**
	 * First contact with an empty canvas.
	 *
	 * A blank infinite surface tells you nothing about itself, so this names the
	 * loop in the order you actually do it. It is `pointer-events-none` so it
	 * never gets in the way of the first gesture it is describing, and it
	 * disappears the moment anything exists.
	 */
	const steps = [
		{ n: '01', title: 'Put things down', body: 'Double-click anywhere to make a note. Messy is fine.' },
		{ n: '02', title: 'Move them until it makes sense', body: 'That rearranging is the thinking. Nothing is recorded as order yet.' },
		{ n: '03', title: 'Draw a frame around a cluster', body: 'Press F and drag. A frame is one section of the document — and the only thing that has an edge you can finish.' },
		{ n: '04', title: 'Read it back', body: 'Document assembles what is inside the frames, in their order. You never write it by hand.' }
	];
</script>

<div class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center p-10">
	<div class="max-w-lg">
		<p class="label mb-6">Empty topic</p>

		<ol class="flex flex-col gap-5">
			{#each steps as step (step.n)}
				<li class="grid grid-cols-[auto_1fr] gap-x-4">
					<span class="font-mono text-[10px] opacity-30 tabular-nums">{step.n}</span>
					<div>
						<h3 class="mb-0.5 text-[13px] font-medium">{step.title}</h3>
						<p class="text-muted-foreground text-[12px] leading-relaxed">{step.body}</p>
					</div>
				</li>
			{/each}
		</ol>

		<div class="mt-7 border-t pt-4" style="border-color: var(--hairline)">
			<p class="text-muted-foreground text-[11px] leading-relaxed">
				Nothing here is required and nothing is ever blocked. Notes can sit outside every
				frame forever — they just stay out of the document.
			</p>
		</div>
	</div>
</div>
