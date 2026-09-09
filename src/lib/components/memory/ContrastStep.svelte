<script lang="ts">
	/**
	 * Step 4: the source is revealed and compared.
	 *
	 * This is the only writer of `last_checked_at`, and it is deliberately a
	 * separate step from the save — you can close the sheet here and never
	 * contrast, which is exactly why recall and accuracy are allowed to diverge.
	 * The app records that divergence as a small mark rather than nagging.
	 *
	 * Two outcomes, and neither is scored. "I left things out" is the interesting
	 * one: what you missed becomes a Question at the highest origin, because a
	 * gap you found reproducing something outranks anything you jotted down.
	 *
	 * You type the gaps yourself. Nothing is diffed or matched — a machine
	 * deciding what you left out would be judging your recall, and it would be
	 * wrong about Spanish besides.
	 */
	import { Button } from '$lib/components/ui/button';
	import type { Note } from '$lib/types';
	import '$lib/components/edra/shadcn/editor.css';

	let {
		note,
		outgoing,
		onresolve
	}: {
		note: Note;
		/** What was there before the blind rewrite, if anything. */
		outgoing: { title: string; html: string; text: string } | null;
		onresolve: (gaps: string[]) => void;
	} = $props();

	let listing = $state(false);
	let gapText = $state('');

	const gaps = $derived(
		gapText
			.split('\n')
			.map((line) => line.trim())
			.filter(Boolean)
	);
</script>

<div class="flex min-h-0 grow flex-col">
	<div class="grid min-h-0 grow grid-cols-2">
		<section class="flex min-h-0 flex-col border-r" style="border-color: var(--hairline)">
			<header class="shrink-0 border-b px-5 py-2.5" style="border-color: var(--hairline)">
				<span class="label">What you wrote</span>
			</header>
			<div class="selectable min-h-0 grow overflow-y-auto px-5 py-4">
				{#if note.body_html.trim()}
					<div class="tiptap prose-doc">{@html note.body_html}</div>
				{:else}
					<p class="text-muted-foreground text-[12px] italic">Nothing.</p>
				{/if}
			</div>
		</section>

		<section class="flex min-h-0 flex-col">
			<header class="shrink-0 border-b px-5 py-2.5" style="border-color: var(--hairline)">
				<span class="label">{outgoing ? 'Previous version' : 'Source'}</span>
			</header>
			<div class="selectable bg-muted/25 min-h-0 grow overflow-y-auto px-5 py-4">
				{#if outgoing && outgoing.html.trim()}
					<div class="tiptap prose-doc">{@html outgoing.html}</div>
				{:else}
					<p class="text-muted-foreground text-[12px] italic">
						There was nothing here before — this was the first time you wrote it.
					</p>
				{/if}
			</div>
		</section>
	</div>

	{#if listing}
		<div class="shrink-0 border-t px-5 py-3" style="border-color: var(--hairline)">
			<p class="label mb-2">What was missing — one per line</p>
			<!-- svelte-ignore a11y_autofocus -->
			<textarea
				autofocus
				bind:value={gapText}
				rows="3"
				class="w-full resize-none rounded border bg-transparent p-2.5 text-[13px] leading-relaxed outline-none"
				placeholder="Each line becomes an open question."
			></textarea>
		</div>
	{/if}

	<footer
		class="flex shrink-0 items-center gap-2 border-t px-5 py-2.5"
		style="border-color: var(--hairline)"
	>
		{#if listing}
			<Button size="sm" class="h-7 text-[11px]" onclick={() => onresolve(gaps)}>
				File {gaps.length || 'no'}
				{gaps.length === 1 ? 'question' : 'questions'}
			</Button>
			<Button
				variant="ghost"
				size="sm"
				class="h-7 text-[11px]"
				onclick={() => {
					listing = false;
					gapText = '';
				}}
			>
				Back
			</Button>
		{:else}
			<Button size="sm" class="h-7 text-[11px]" onclick={() => onresolve([])}>It matches</Button>
			<Button
				variant="outline"
				size="sm"
				class="h-7 text-[11px]"
				onclick={() => (listing = true)}
			>
				I left things out
			</Button>
		{/if}

		<span class="text-muted-foreground ml-auto text-[10px]">
			Closing without answering leaves this note marked as never contrasted.
		</span>
	</footer>
</div>
