<script lang="ts">
	/**
	 * The daily review.
	 *
	 * Cold by construction: the body is not rendered until you have answered. It
	 * is not hidden with CSS and not collapsed — it is absent from the DOM, the
	 * same reasoning as memory mode. If it can be looked at, it will be.
	 *
	 * A card is a projection of an atomic note: the title is the front, the body
	 * is the back. Nothing was authored, so nothing can drift from its source,
	 * and editing a card means editing the note.
	 *
	 * The queue is a day's worth and the backlog behind it is never shown — not
	 * as a number, not as "overdue", not anywhere. Coming back after a break to
	 * a four-figure count is the classic reason people abandon review, and it is
	 * the same accumulating-debt feeling the rest of the app refuses.
	 */
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import * as ipc from '$lib/ipc';
	import { chrome } from '$lib/state/chrome.svelte';
	import BarTitle from '$lib/components/BarTitle.svelte';
	import { session } from '$lib/state/session.svelte';
	import type { Grade, ReviewCard } from '$lib/types';
	import { GRADE_LABEL, STATE_MEANING } from '$lib/types';
	import { Button } from '$lib/components/ui/button';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import '$lib/components/edra/shadcn/editor.css';

	let queue = $state<ReviewCard[]>([]);
	let index = $state(0);
	let revealed = $state(false);
	let loading = $state(true);
	let done = $state(0);

	const card = $derived(queue[index] ?? null);

	/** The card's note has moved since it was recalled, so it may be wrong now. */
	const drifted = $derived(card?.state === 'edited_since_recall');

	onMount(async () => {
		queue = await ipc.reviewQueue();
		loading = false;
	});

	function next() {
		revealed = false;
		index += 1;
		done += 1;
		session.refreshReviewCount();
	}

	async function grade(value: Grade) {
		if (!card) return;
		const id = card.id;
		next();
		await ipc.gradeCard(id, value);
		if (value === 'again') session.openQuestions += 1;
	}

	async function resolveDrift(stillHolds: boolean) {
		if (!card) return;
		const id = card.id;
		next();
		await ipc.resolveDriftedCard(id, stillHolds);
		if (!stillHolds) session.openQuestions += 1;
	}

	const grades: Grade[] = ['again', 'hard', 'good', 'easy'];

	function onkeydown(event: KeyboardEvent) {
		if (!card || event.ctrlKey || event.metaKey) return;

		if (!revealed && (event.key === ' ' || event.key === 'Enter')) {
			event.preventDefault();
			revealed = true;
			return;
		}

		if (revealed && !drifted && ['1', '2', '3', '4'].includes(event.key)) {
			event.preventDefault();
			grade(grades[Number(event.key) - 1]);
		}
	}

	$effect(() => chrome.claim(bar));
</script>

{#snippet bar()}<BarTitle>Review</BarTitle>{/snippet}

<svelte:window {onkeydown} />
<svelte:head><title>Review — study</title></svelte:head>

<div class="flex h-full flex-col">
	{#if loading}
		<div class="grow"></div>
	{:else if !card}
		<!-- Finished, or nothing due. Both are simply true; neither is praised
		     and neither is a streak. -->
		<div class="mx-auto flex max-w-md grow flex-col justify-center gap-4 px-8">
			<p class="label">Review</p>
			<p class="text-[14px] leading-relaxed">
				{#if done > 0}
					{done} card{done === 1 ? '' : 's'} done. That is today's queue.
				{:else}
					Nothing due today.
				{/if}
			</p>
			<p class="text-muted-foreground text-[12px] leading-relaxed">
				A card exists once a note has been reproduced from memory. Write something in
				memory mode and it will turn up here.
			</p>
			<div class="flex gap-2">
				<Button size="sm" class="h-8 text-[12px]" onclick={() => goto('/')}>
					Open questions
				</Button>
			</div>
		</div>
	{:else}
		<!-- Progress through today's queue only. Never the backlog behind it. -->
		<div
			class="flex shrink-0 items-center gap-3 border-b px-6 py-2.5"
			style="border-color: var(--hairline)"
		>
			<span class="label">Review</span>
			{#if card.topic_title}
				<span class="text-muted-foreground text-[11px]">{card.topic_title}</span>
			{/if}
			<span class="ml-auto font-mono text-[10px] tabular-nums opacity-40">
				{index + 1} / {queue.length}
			</span>
		</div>

		<div class="mx-auto flex w-full max-w-2xl grow flex-col justify-center gap-6 px-8 py-10">
			{#if drifted}
				<!-- Not suspended. Suspension plus silent decay would switch the queue
				     off without telling you; this card leaves by being acted on
				     instead, so it can never become an ungradeable card that returns
				     forever. -->
				<div
					class="rounded-md border px-4 py-3"
					style="border-color: var(--state-action); background: var(--state-action-soft)"
				>
					<p class="mb-1 flex items-center gap-2 text-[12px] font-medium">
						<StateMark state={card.state} visible={card.visible} />
						This note changed after you last reproduced it
					</p>
					<p class="text-[11.5px] leading-relaxed opacity-80">
						Grading it would test text you may no longer stand behind, so it is not
						graded. Say whether it still holds.
					</p>
				</div>
			{/if}

			<h1 class="font-serif text-[26px] leading-tight font-semibold tracking-tight">
				{card.title || 'Untitled'}
			</h1>

			{#if revealed}
				<div class="selectable">
					{#if card.body_html.trim()}
						<div class="tiptap prose-doc">{@html card.body_html}</div>
					{:else}
						<p class="text-muted-foreground text-[13px] italic">This note has no body.</p>
					{/if}
				</div>

				<p class="text-muted-foreground flex items-center gap-2 text-[11px]">
					<StateMark state={card.state} visible={card.visible} neverContrasted={card.never_contrasted} />
					{STATE_MEANING[card.state]}
				</p>
			{:else}
				<p class="text-muted-foreground text-[12.5px] leading-relaxed">
					Say it, then reveal. <kbd class="font-mono">Space</kbd>
				</p>
			{/if}
		</div>

		<footer
			class="flex shrink-0 items-center gap-2 border-t px-6 py-3"
			style="border-color: var(--hairline)"
		>
			{#if !revealed}
				<Button size="sm" class="h-8 text-[12px]" onclick={() => (revealed = true)}>
					Reveal
				</Button>
			{:else if drifted}
				<Button size="sm" class="h-8 text-[12px]" onclick={() => resolveDrift(true)}>
					It still holds
				</Button>
				<Button
					variant="outline"
					size="sm"
					class="h-8 text-[12px]"
					onclick={() => resolveDrift(false)}
				>
					Needs work
				</Button>
				<span class="text-muted-foreground ml-auto text-[10px]">
					Either way this does not count against the card.
				</span>
			{:else}
				{#each grades as g, i (g)}
					<Button
						variant={g === 'good' ? 'default' : 'outline'}
						size="sm"
						class="h-8 text-[12px]"
						onclick={() => grade(g)}
					>
						{GRADE_LABEL[g]}
						<kbd class="ml-1.5 font-mono text-[9px] opacity-45">{i + 1}</kbd>
					</Button>
				{/each}
			{/if}
		</footer>
	{/if}
</div>
