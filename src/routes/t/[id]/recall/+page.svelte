<script lang="ts">
	/**
	 * Reconstruction, in three rounds.
	 *
	 * Round one is free recall and nothing else. No list, no autocomplete, no
	 * hints — and no count of how many concepts are waiting, because knowing
	 * there are twelve turns free recall into a bounded task you pad toward. The
	 * count appears after you submit, never during.
	 *
	 * Round two reveals the real set and you mark yourself. Nothing is matched
	 * automatically: exact comparison fails constantly across `SRS` and
	 * `repetición espaciada`, fuzzy comparison is a machine guessing at your
	 * recall, and both would be judging the thing this round exists to measure.
	 * Recall integrity survives because the typing already happened.
	 *
	 * Round three is optional and is the only test of structure. The frames come
	 * back empty *and unlabelled*: you group the concepts, then name each group
	 * from what you put in it. Labels shown up front would give away the
	 * relations; unlabelled boxes with no naming step would test nothing.
	 *
	 * The misses become one Question for the session, not one per miss — sixteen
	 * near-identical questions at the top priority would train you to ignore the
	 * bucket that is supposed to mean "most important".
	 */
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import * as ipc from '$lib/ipc';
	import { session } from '$lib/state/session.svelte';
	import type { CanvasSnapshot, Frame, Note, Topic } from '$lib/types';
	import { Button } from '$lib/components/ui/button';
	import ArrowLeftIcon from '@lucide/svelte/icons/arrow-left';

	type Round = 'writing' | 'marking' | 'placing' | 'done';

	const topicId = $derived(page.params.id!);

	let topic = $state<Topic | null>(null);
	let snapshot = $state<CanvasSnapshot | null>(null);
	let allNotes = $state<Note[]>([]);

	let round = $state<Round>('writing');
	let typed = $state('');
	/** note id -> recalled. Everything starts unmarked; you claim the hits. */
	let hits = $state<Record<string, boolean>>({});
	let filed = $state(0);

	/** Round three: note id -> which empty box it was put in. */
	let placements = $state<Record<string, number>>({});
	let boxNames = $state<Record<number, string>>({});
	let selected = $state<string | null>(null);
	let revealed = $state(false);

	onMount(async () => {
		const [t, snap, notes] = await Promise.all([
			ipc.openTopic(topicId),
			ipc.canvasSnapshot(topicId),
			ipc.topicNotes(topicId)
		]);
		topic = t;
		snapshot = snap;
		allNotes = notes;
		session.noteVisit(t);
	});

	/**
	 * The real set: notes committed to a frame. A note loose on the canvas is not
	 * part of the topic's structure yet, so it is not something you failed to
	 * recall.
	 */
	const realSet = $derived.by(() => {
		if (!snapshot) return [] as { note: Note; frameId: string }[];

		return snapshot.nodes
			.filter((n) => n.kind === 'note' && n.frame_id && n.note_id)
			.map((n) => ({
				note: allNotes.find((x) => x.id === n.note_id),
				frameId: n.frame_id!
			}))
			.filter((x): x is { note: Note; frameId: string } => Boolean(x.note));
	});

	const frames = $derived(
		[...(snapshot?.frames ?? [])].sort((a, b) => a.order_index - b.order_index)
	);

	const written = $derived(typed.split('\n').filter((l) => l.trim()).length);
	const recalled = $derived(realSet.filter((r) => hits[r.note.id]));
	const missed = $derived(realSet.filter((r) => !hits[r.note.id]));

	function reveal() {
		hits = Object.fromEntries(realSet.map((r) => [r.note.id, false]));
		round = 'marking';
	}

	/** One Question per session, carrying its missed concepts. */
	async function fileGaps(next: Round) {
		if (missed.length > 0) {
			const names = missed.map((m) => m.note.title || 'Untitled').join(', ');
			await ipc.createQuestion(
				`What did I miss in ${topic?.title ?? 'this topic'}? — ${names}`,
				'recall_gap',
				topicId
			);
			session.openQuestions += 1;
			filed = missed.length;
		}

		// Only what you recalled goes into round three; you cannot arrange
		// something you could not bring back.
		placements = {};
		boxNames = {};
		selected = null;
		revealed = false;
		round = next;
	}

	function place(boxIndex: number) {
		if (!selected) return;
		placements[selected] = boxIndex;
		selected = null;
	}

	const unplaced = $derived(recalled.filter((r) => placements[r.note.id] === undefined));

	function boxContents(index: number) {
		return recalled.filter((r) => placements[r.note.id] === index);
	}

	function realContents(frame: Frame) {
		return realSet.filter((r) => r.frameId === frame.id);
	}
</script>

<svelte:head><title>Reconstruct — {topic?.title ?? ''} — study</title></svelte:head>

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-3xl flex-col gap-6 px-8 py-10">
		<header class="flex flex-col gap-1">
			<a
				href="/t/{topicId}"
				class="text-muted-foreground hover:text-foreground mb-2 flex w-fit items-center gap-1.5 text-[11px] transition-colors"
			>
				<ArrowLeftIcon class="size-3" />
				Back to the canvas
			</a>

			<p class="label">
				{#if round === 'writing'}Round one — recall{:else if round === 'marking'}Round two — compare{:else if round === 'placing'}Round three — arrange{:else}Done{/if}
			</p>
			<h1 class="text-xl font-medium tracking-tight">{topic?.title ?? ''}</h1>
		</header>

		<!-- ROUND ONE ---------------------------------------------------- -->
		{#if round === 'writing'}
			<p class="text-muted-foreground text-[13px] leading-relaxed">
				The canvas is empty. Write everything you can remember being on it, one per line.
				Nothing is checked until you say you are finished.
			</p>

			<!-- svelte-ignore a11y_autofocus -->
			<textarea
				autofocus
				bind:value={typed}
				rows="14"
				spellcheck="false"
				autocomplete="off"
				autocapitalize="off"
				placeholder="One concept per line…"
				class="w-full resize-none rounded-md border bg-transparent p-4 text-[14px] leading-relaxed outline-none"
			></textarea>

			<!-- No count of the real set here. Knowing the target bounds the task. -->
			<div class="flex items-center gap-3">
				<Button size="sm" class="h-8 text-[12px]" disabled={written === 0} onclick={reveal}>
					I am finished — show me
				</Button>
			</div>

		<!-- ROUND TWO ---------------------------------------------------- -->
		{:else if round === 'marking'}
			<p class="text-muted-foreground text-[13px] leading-relaxed">
				You wrote {written}. The topic holds {realSet.length}. Tick what you had — you are
				the judge here, and nothing compares your words to these for you.
			</p>

			<div class="grid gap-6 sm:grid-cols-[1fr_1fr]">
				<section class="flex flex-col gap-1.5">
					<h2 class="label">What you wrote</h2>
					<p
						class="selectable rounded-md border p-3 text-[13px] leading-relaxed whitespace-pre-wrap"
						style="border-color: var(--hairline)"
					>
						{typed}
					</p>
				</section>

				<section class="flex flex-col gap-1.5">
					<h2 class="label">What was there</h2>
					<ul class="flex flex-col">
						{#each realSet as item (item.note.id)}
							<li>
								<label
									class="hover:bg-accent flex cursor-pointer items-center gap-2.5 rounded px-2 py-1.5"
								>
									<input
										type="checkbox"
										bind:checked={hits[item.note.id]}
										class="accent-foreground size-3.5"
									/>
									<span
										class="text-[13px]"
										style={hits[item.note.id] ? 'color: var(--state-recalled)' : ''}
									>
										{item.note.title || 'Untitled'}
									</span>
								</label>
							</li>
						{:else}
							<li class="text-muted-foreground px-2 py-2 text-[12px] leading-relaxed">
								Nothing is committed to a frame in this topic, so there was no structure
								to reconstruct yet.
							</li>
						{/each}
					</ul>
				</section>
			</div>

			<div
				class="flex items-center gap-3 border-t pt-4"
				style="border-color: var(--hairline)"
			>
				<Button
					size="sm"
					class="h-8 text-[12px]"
					disabled={recalled.length === 0 || frames.length === 0}
					onclick={() => fileGaps('placing')}
				>
					Arrange what came back
				</Button>
				<Button
					variant="outline"
					size="sm"
					class="h-8 text-[12px]"
					onclick={() => fileGaps('done')}
				>
					Stop here
				</Button>
				<span class="text-muted-foreground ml-auto text-[11px]">
					{recalled.length} of {realSet.length} came back
				</span>
			</div>

		<!-- ROUND THREE -------------------------------------------------- -->
		{:else if round === 'placing'}
			<p class="text-muted-foreground text-[13px] leading-relaxed">
				{#if revealed}
					Here is how they were actually grouped. Nothing is scored — the point was
					whether you could name what you had gathered.
				{:else}
					The frames come back empty and unnamed. Click a concept, then a box. When a box
					has something in it, say what it is.
				{/if}
			</p>

			{#if !revealed}
				{#if unplaced.length > 0}
					<section class="flex flex-col gap-1.5">
						<h2 class="label">Still to place</h2>
						<div class="flex flex-wrap gap-1.5">
							{#each unplaced as item (item.note.id)}
								<button
									type="button"
									onclick={() => (selected = selected === item.note.id ? null : item.note.id)}
									class="rounded border px-2 py-1 text-[12px] transition-colors
										{selected === item.note.id ? 'bg-foreground text-background' : 'hover:bg-accent'}"
									style="border-color: var(--hairline-strong)"
								>
									{item.note.title || 'Untitled'}
								</button>
							{/each}
						</div>
					</section>
				{/if}

				<div class="grid gap-3 sm:grid-cols-2">
					{#each frames as _frame, i (i)}
						<section
							class="flex min-h-28 flex-col gap-2 rounded-lg border border-dashed p-3 transition-colors"
							style="border-color: {selected ? 'var(--foreground)' : 'var(--frame-line)'}"
						>
							<button
								type="button"
								onclick={() => place(i)}
								disabled={!selected}
								class="text-left disabled:cursor-default"
							>
								<span class="label">
									{selected ? 'put it here' : `group ${i + 1}`}
								</span>
							</button>

							<div class="flex flex-wrap gap-1.5">
								{#each boxContents(i) as item (item.note.id)}
									<button
										type="button"
										title="Take it back out"
										onclick={() => {
											delete placements[item.note.id];
											placements = { ...placements };
										}}
										class="bg-card rounded border px-2 py-1 text-[12px]"
										style="border-color: var(--hairline)"
									>
										{item.note.title || 'Untitled'}
									</button>
								{/each}
							</div>

							{#if boxContents(i).length > 0}
								<!-- Naming the group from its contents is the actual test. -->
								<input
									bind:value={boxNames[i]}
									placeholder="What is this group?"
									class="mt-auto w-full rounded border bg-transparent px-2 py-1 text-[12px] outline-none"
									style="border-color: var(--hairline)"
								/>
							{/if}
						</section>
					{/each}
				</div>

				<div class="flex items-center gap-3 border-t pt-4" style="border-color: var(--hairline)">
					<Button size="sm" class="h-8 text-[12px]" onclick={() => (revealed = true)}>
						Show me how they were grouped
					</Button>
				</div>
			{:else}
				<div class="grid gap-3 sm:grid-cols-2">
					{#each frames as frame, i (frame.id)}
						<section
							class="flex flex-col gap-2 rounded-lg border p-3"
							style="border-color: var(--hairline)"
						>
							<div class="flex items-baseline gap-2">
								<span class="label">yours</span>
								<span class="text-[12px] {boxNames[i]?.trim() ? '' : 'opacity-40 italic'}">
									{boxNames[i]?.trim() || 'unnamed'}
								</span>
							</div>
							<div class="flex flex-wrap gap-1.5">
								{#each boxContents(i) as item (item.note.id)}
									<span class="rounded border px-2 py-1 text-[11px]" style="border-color: var(--hairline)">
										{item.note.title || 'Untitled'}
									</span>
								{/each}
							</div>

							<div class="mt-1 flex items-baseline gap-2 border-t pt-2" style="border-color: var(--hairline)">
								<span class="label">actually</span>
								<span class="text-[12px] {frame.label ? '' : 'opacity-40 italic'}">
									{frame.label || 'untitled section'}
								</span>
							</div>
							<div class="flex flex-wrap gap-1.5">
								{#each realContents(frame) as item (item.note.id)}
									<span
										class="rounded border px-2 py-1 text-[11px]"
										style="border-color: var(--hairline)"
									>
										{item.note.title || 'Untitled'}
									</span>
								{/each}
							</div>
						</section>
					{/each}
				</div>

				<div class="flex items-center gap-3 border-t pt-4" style="border-color: var(--hairline)">
					<Button size="sm" class="h-8 text-[12px]" onclick={() => (round = 'done')}>
						Finish
					</Button>
				</div>
			{/if}

		<!-- SUMMARY ------------------------------------------------------ -->
		{:else}
			<div class="flex flex-col gap-4">
				<p class="text-[14px] leading-relaxed">
					{recalled.length} of {realSet.length} came back.
					{#if filed > 0}
						The {filed} that did not are gathered into one open question.
					{:else}
						Nothing was left over.
					{/if}
				</p>

				<div class="flex gap-2">
					<Button size="sm" class="h-8 text-[12px]" onclick={() => goto('/questions')}>
						Go to the queue
					</Button>
					<Button
						variant="outline"
						size="sm"
						class="h-8 text-[12px]"
						onclick={() => goto(`/t/${topicId}`)}
					>
						Back to the canvas
					</Button>
				</div>
			</div>
		{/if}
	</div>
</div>
