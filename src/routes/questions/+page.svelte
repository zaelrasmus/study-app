<script lang="ts">
	/**
	 * The question queue.
	 *
	 * **It leads with one question, not a list.**
	 *
	 * A list of open questions is a list of things you have not done, and that
	 * is exactly the shape this app refuses everywhere else. So the top of the
	 * queue is lifted out and presented as *the* thing to answer — one concrete
	 * act, at a size you can read from across the desk. The rest sits below it,
	 * quieter, for when you want to choose rather than be handed something.
	 *
	 * Priority comes from origin, and origin is visible: an order sorted by an
	 * invisible key is indistinguishable from no order at all.
	 *
	 * The queue arrives already ordered from Rust — origin first, then the
	 * topic's last-visited time, then creation. Nothing here re-sorts it. Topic
	 * and age are shown as secondary metadata precisely because they are *not*
	 * the sort key: a question about what you are working on now is answerable,
	 * one from eight months ago is not, but neither fact ever promotes a band.
	 */
	import { fade, slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import * as ipc from '$lib/ipc';
	import { slot } from '$lib/state/slot.svelte';
	import { chrome } from '$lib/state/chrome.svelte';
	import BarTitle from '$lib/components/BarTitle.svelte';
	import { session } from '$lib/state/session.svelte';
	import type { Note, Question, QuestionOrigin, Topic } from '$lib/types';
	import { cardFace, ORIGIN_LABEL, ORIGIN_TAG, ORIGIN_WHY } from '$lib/types';
	import { drag } from '$lib/state/drag.svelte';
	import { data } from '$lib/state/data.svelte';
	import ChevronIcon from '@lucide/svelte/icons/chevron-right';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import NoteEditor from '$lib/components/NoteEditor.svelte';

	let questions = $state<Question[]>([]);
	let topics = $state<Topic[]>([]);
	let editingNoteId = $state<string | null>(null);
	/** Folded by default: the screen is about the question in front of you. */
	let showInbox = $state(false);

	// An effect, not `onMount`: this screen used to load once and then sit there
	// while you answered a question on a board or captured another one.
	$effect(() => {
		data.questions;
		data.boards;

		Promise.all([ipc.questionQueue(), ipc.allTopics()]).then(([q, t]) => {
			questions = q;
			topics = t;
		});
	});

	/** The one at the top of the queue. The screen is built around it. */
	const next = $derived(questions[0] ?? null);

	/** Everything behind it, still in bands. Empty bands are not shown. */
	const bands = $derived(
		(['recall_gap', 'review_failure', 'capture'] as QuestionOrigin[])
			.map((origin) => ({
				origin,
				items: questions.slice(1).filter((q) => q.origin === origin)
			}))
			.filter((band) => band.items.length > 0)
	);

	function topicTitle(id: string | null) {
		return id ? (topics.find((t) => t.id === id)?.title ?? null) : null;
	}

	/** Coarse on purpose: an exact timestamp would invite reading it as a rank. */
	function age(iso: string) {
		const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
		if (days < 1) return 'today';
		if (days === 1) return 'yesterday';
		if (days < 30) return `${days}d`;
		if (days < 365) return `${Math.floor(days / 30)}mo`;
		return `${Math.floor(days / 365)}y`;
	}

	/** One click, no confirmation. If abandoning costs anything, you hoard. */
	async function drop(question: Question, status: 'abandoned' | 'dissolved') {
		questions = questions.filter((q) => q.id !== question.id);
		session.openQuestions -= 1;
		await ipc.setQuestionStatus(question.id, status);
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
		editingNoteId = note.id;
	}

	/**
	 * A question is a thing you can put somewhere.
	 *
	 * Dropping one on a board attaches it there and stands it next to the cards
	 * that might answer it, which is also how a doubt caught with nothing open
	 * finds a home later.
	 */
	function grab(event: PointerEvent, question: Question) {
		drag.begin({ kind: 'question', questionId: question.id, label: question.text }, event);
	}

	function onSaved(_note: Note) {
		session.refresh();
	}

	// The inspector borrows this screen's opener, so a backlink is followable
	// from the panel without the panel needing to know how notes get opened.
	$effect(() => {
		slot.openNote = (id: string) => (editingNoteId = id);
		return () => (slot.openNote = undefined);
	});

	$effect(() => chrome.claim(bar));
</script>

{#snippet bar()}<BarTitle>Questions</BarTitle>{/snippet}

<svelte:head><title>Questions — study</title></svelte:head>

{#snippet meta(question: Question)}
	{@const topic = topicTitle(question.topic_id)}
	<div class="text-muted-foreground flex items-center gap-2 font-mono text-[9.5px]">
		<span class="tracking-wider uppercase opacity-70">{ORIGIN_TAG[question.origin]}</span>
		{#if topic}
			<span class="opacity-30">·</span>
			<span class="truncate">{topic}</span>
		{/if}
		<span class="opacity-30">·</span>
		<span>{age(question.created_at)}</span>
	</div>
{/snippet}

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-2xl flex-col px-10 py-14">
		{#if next}
			<!-- The one to answer. Serif, large, alone on its own field: this is
			     content rather than chrome, and it is the only thing being asked
			     of you right now. -->
			<section in:fade={{ duration: 200 }}>
				<p class="label mb-5">Next</p>

				{#key next.id}
					<div in:slide={{ duration: 200, easing: cubicOut }}>
						<h1 class="font-serif text-[27px] leading-[1.35] tracking-[-0.01em]">
							{next.text}
						</h1>

						<div class="mt-4">
							{@render meta(next)}
						</div>

						<p class="text-muted-foreground mt-6 max-w-md text-[12.5px] leading-relaxed">
							{ORIGIN_WHY[next.origin]}
						</p>

						<div class="mt-7 flex items-center gap-2">
							<button
								type="button"
								onclick={() => answer(next)}
								class="bg-foreground text-background rounded-md px-4 py-2 text-[12.5px] font-medium transition-opacity hover:opacity-85"
							>
								Answer it
							</button>
							<button
								type="button"
								onclick={() => drop(next, 'dissolved')}
								title="It rested on a wrong premise, so there is nothing to answer"
								class="hover:bg-accent rounded-md border px-3 py-2 text-[12.5px] transition-colors"
								style="border-color: var(--hairline)"
							>
								Dissolve
							</button>
							<button
								type="button"
								onclick={() => drop(next, 'abandoned')}
								class="hover:bg-accent text-muted-foreground rounded-md px-3 py-2 text-[12.5px] transition-colors"
							>
								Drop
							</button>
						</div>
					</div>
				{/key}
			</section>

			{#if bands.length > 0}
				<div class="mt-14 border-t" style="border-color: var(--hairline)"></div>
			{/if}
		{:else}
			<section class="py-6">
				<h1 class="font-serif text-[27px] leading-tight tracking-[-0.01em]">Nothing open.</h1>
				<p class="text-muted-foreground mt-4 max-w-md text-[13px] leading-relaxed">
					Questions arrive on their own: write something from memory and the gaps become
					questions. Or capture one with <kbd class="font-mono text-[12px]">Ctrl N</kbd>,
					then <kbd class="font-mono text-[12px]">Ctrl Enter</kbd>.
				</p>
			</section>
		{/if}

		{#each bands as band (band.origin)}
			<section class="mt-10 flex flex-col">
				<header class="mb-3 flex items-baseline gap-3">
					<h2 class="label">{ORIGIN_LABEL[band.origin]}</h2>
					<span class="text-muted-foreground truncate text-[11.5px]">
						{ORIGIN_WHY[band.origin]}
					</span>
					<span class="ml-auto font-mono text-[10px] tabular-nums opacity-25">
						{band.items.length}
					</span>
				</header>

				{#each band.items as question (question.id)}
					<!-- A card, not a row: the same thing the library shows, and the
					     same thing you can pick up and put on a board. -->
					<div
						role="listitem"
						onpointerdown={(e) => grab(e, question)}
						title="Drag onto a board to put it beside what might answer it"
						class="group bg-card hover:border-foreground/25 mb-2 flex cursor-grab touch-none
							items-start gap-4 rounded-lg p-4 transition-colors active:cursor-grabbing"
						style="border: 1px dashed var(--hairline)"
					>
						<div class="flex min-w-0 grow flex-col gap-2">
							<p class="font-serif text-[15px] leading-relaxed">{question.text}</p>
							{@render meta(question)}
						</div>

						<div
							class="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
						>
							<button
								type="button"
								onclick={() => answer(question)}
								class="hover:bg-accent rounded px-2 py-1 text-[11.5px] font-medium transition-colors"
							>
								Answer
							</button>
							<button
								type="button"
								onclick={() => drop(question, 'dissolved')}
								title="It rested on a wrong premise"
								class="hover:bg-accent text-muted-foreground rounded px-2 py-1 text-[11.5px] transition-colors"
							>
								Dissolve
							</button>
							<button
								type="button"
								onclick={() => drop(question, 'abandoned')}
								class="hover:bg-accent text-muted-foreground rounded px-2 py-1 text-[11.5px] transition-colors"
							>
								Drop
							</button>
						</div>
					</div>
				{/each}
			</section>
		{/each}

		{#if session.inbox.length > 0}
			<section class="mt-14 flex flex-col border-t pt-9" style="border-color: var(--hairline)">
				<!-- Folded by default. This screen is about the question in front of
				     you; a grid of unrelated captures under it is the pile the whole
				     app exists to avoid. -->
				<button
					type="button"
					onclick={() => (showInbox = !showInbox)}
					class="hover:bg-accent -mx-2 flex items-center gap-2 self-start rounded-md px-2 py-1 transition-colors"
				>
					<ChevronIcon
						class="size-3 opacity-35 transition-transform"
						style="transform: rotate({showInbox ? 90 : 0}deg)"
					/>
					<span class="label">Unplaced</span>
					<span class="font-mono text-[10px] tabular-nums opacity-30">
						{session.inbox.length}
					</span>
				</button>

				{#if showInbox}
				<p class="text-muted-foreground mt-2 mb-3 text-[12.5px] leading-relaxed">
					Captured but not on any board yet. There is no rush — these keep.
				</p>

				<div
					class="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-2.5"
					transition:slide={{ duration: 160, easing: cubicOut }}
				>
					{#each session.inbox as note (note.id)}
						<button
							type="button"
							class="bg-card hover:border-foreground/25 flex items-start gap-2 rounded-lg border p-3 text-left transition-colors"
							style="border-color: var(--hairline)"
							onclick={() => (editingNoteId = note.id)}
						>
							<StateMark
								state={note.state}
								visible={note.visible}
								neverContrasted={note.never_contrasted}
								class="mt-[5px] shrink-0"
							/>
							<span class="min-w-0 text-[12.5px] leading-snug">
								{cardFace(note) || 'Untitled'}
							</span>
						</button>
					{/each}
				</div>
				{/if}
			</section>
		{/if}
	</div>
</div>

{#if editingNoteId}
	{#key editingNoteId}
		<NoteEditor noteId={editingNoteId} onclose={() => (editingNoteId = null)} onsaved={onSaved} />
	{/key}
{/if}
