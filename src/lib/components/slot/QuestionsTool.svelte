<script lang="ts">
	/**
	 * Doubts, wherever you are.
	 *
	 * Questions are not a study mechanic — they are how thinking happens, and
	 * they surface while you write messily as much as while you read.
	 *
	 * **It used to show only the board's own, and hide the rest.** The reasoning
	 * was that adopting unattached questions would make this a second copy of the
	 * Questions screen. That was wrong in the case that matters most: a doubt you
	 * had *while thinking about something* has no board yet, and hiding it means
	 * the panel is empty at exactly the moment you wanted it. So everything is
	 * reachable, and the scope says which slice you are looking at.
	 *
	 * Drag one onto a canvas to put the doubt beside the cards that might answer
	 * it — which is also what attaches it to that board.
	 */
	import * as ipc from '$lib/ipc';
	import { session } from '$lib/state/session.svelte';
	import { drag } from '$lib/state/drag.svelte';
	import { data } from '$lib/state/data.svelte';
	import type { Question, QuestionOrigin } from '$lib/types';
	import { ORIGIN_LABEL, ORIGIN_TAG } from '$lib/types';
	import { menu } from '$lib/state/menu.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import PanelCard from './PanelCard.svelte';

	let { topicId = null }: { topicId?: string | null } = $props();

	type Scope = 'board' | 'loose' | 'all';

	/**
	 * On a board, its own come first; elsewhere there is no "board" to mean.
	 *
	 * The initial value is deliberate: this is the starting choice, and the
	 * effect below corrects it if the board goes away while you are looking.
	 */
	// svelte-ignore state_referenced_locally
	let scope = $state<Scope>(topicId ? 'board' : 'loose');
	let questions = $state<Question[]>([]);

	const scopes: { id: Scope; label: string; hint: string }[] = $derived([
		...(topicId
			? [{ id: 'board' as const, label: 'This board', hint: 'Attached to the board in view' }]
			: []),
		{ id: 'loose', label: 'Unattached', hint: 'Caught with no board open — still doubts' },
		{ id: 'all', label: 'All', hint: 'Every open question' }
	]);

	// A board opening or closing changes what the scopes can even mean.
	$effect(() => {
		if (!topicId && scope === 'board') scope = 'loose';
	});

	$effect(() => {
		// A doubt caught anywhere shows up here without reopening the panel.
		data.questions;
		const which = scope;
		const board = topicId;

		const load =
			which === 'board' && board
				? ipc.questionQueue(board)
				: which === 'loose'
					? ipc.unattachedQuestions()
					: ipc.questionQueue();

		load.then((found) => (questions = found));
	});

	const bands = $derived(
		(['recall_gap', 'review_failure', 'capture'] as QuestionOrigin[])
			.map((origin) => ({ origin, items: questions.filter((q) => q.origin === origin) }))
			.filter((band) => band.items.length > 0)
	);

	/** One click, no confirmation, here as everywhere else. */
	async function dropIt(question: Question) {
		questions = questions.filter((q) => q.id !== question.id);
		session.openQuestions -= 1;
		await ipc.setQuestionStatus(question.id, 'abandoned');
	}

	/**
	 * Answering opens a fresh note and links it. Closing a question requires the
	 * note that answers it — the one rule the app genuinely enforces.
	 */
	async function answer(question: Question) {
		const note = await ipc.createNote(question.text.replace(/\?+$/, '').trim());
		await ipc.resolveQuestion(question.id, note.id);

		questions = questions.filter((q) => q.id !== question.id);
		session.openQuestions -= 1;
		slot.openNote?.(note.id);
	}

	function questionMenu(event: MouseEvent, question: Question) {
		menu.show(event, [
			{ label: 'Answer it with a note', onpick: () => answer(question) },
			{ label: 'Drop it', destructive: true, onpick: () => dropIt(question) }
		]);
	}

	function grab(event: PointerEvent, question: Question) {
		drag.begin({ kind: 'question', questionId: question.id, label: question.text }, event);
	}

	const empty = $derived(
		scope === 'board'
			? 'Nothing attached to this board yet. A question caught while it is open lands here — or drag one over.'
			: scope === 'loose'
				? 'Nothing loose. Doubts caught with no board open collect here.'
				: 'Nothing open anywhere.'
	);
</script>

<div class="flex min-h-0 flex-col">
	<div class="flex shrink-0 flex-wrap gap-1 px-3 pt-2.5 pb-2">
		{#each scopes as s (s.id)}
			<button
				type="button"
				onclick={() => (scope = s.id)}
				title={s.hint}
				class="rounded px-2 py-0.5 text-[11px] transition-colors
					{scope === s.id ? 'bg-foreground text-background' : 'hover:bg-accent opacity-45'}"
			>
				{s.label}
			</button>
		{/each}
	</div>

	<div class="min-h-0 grow overflow-y-auto px-2 pb-2">
		{#each bands as band (band.origin)}
			<p class="label mt-3 mb-1.5 px-1 first:mt-0">{ORIGIN_LABEL[band.origin]}</p>

			{#each band.items as question (question.id)}
				<PanelCard
					dashed
					grab
					onpointerdown={(e) => grab(e, question)}
					ondblclick={() => answer(question)}
					oncontextmenu={(e) => questionMenu(e, question)}
					title="Double-click to answer it · drag onto a board to place it"
				>
					<p class="font-serif text-[12.5px] leading-snug">{question.text}</p>

					<div class="flex items-center gap-2">
						<span class="label !text-[8px]">{ORIGIN_TAG[question.origin]}</span>
						<button
							type="button"
							onclick={() => dropIt(question)}
							class="ml-auto font-mono text-[9px] uppercase opacity-35 transition-opacity hover:opacity-100"
						>
							drop
						</button>
					</div>
				</PanelCard>
			{/each}
		{:else}
			<p class="text-muted-foreground px-2 py-3 text-[12px] leading-relaxed">{empty}</p>
		{/each}
	</div>
</div>
