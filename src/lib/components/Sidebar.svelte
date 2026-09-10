<script lang="ts">
	/**
	 * The left sidebar: destinations and recent work, never structure.
	 *
	 * Every item here navigates the *centre*. That is the correction: a nav item
	 * that opened a panel on the far side of the window made the left sidebar a
	 * remote control for the right one, and you had to look somewhere other than
	 * where you clicked. The right slot is now only ever an inspector for what
	 * the centre is already showing.
	 *
	 * No tree, no folders, no paths — and no tab list either, which is the one
	 * part of Heptabase's sidebar deliberately left out: four stacked
	 * organisational layers is the thing this app exists to escape.
	 */
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { slide, fade } from 'svelte/transition';
	import { flip } from 'svelte/animate';
	import { cubicOut } from 'svelte/easing';
	import { session } from '$lib/state/session.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { data } from '$lib/state/data.svelte';
	import { prefs } from '$lib/state/prefs.svelte';
	import * as ipc from '$lib/ipc';
	import type { Topic } from '$lib/types';

	import PlusIcon from '@lucide/svelte/icons/plus';
	import SearchIcon from '@lucide/svelte/icons/search';
	import QuestionsIcon from '@lucide/svelte/icons/circle-help';
	import ReviewIcon from '@lucide/svelte/icons/brain';
	import LibraryIcon from '@lucide/svelte/icons/layout-grid';
	import JournalIcon from '@lucide/svelte/icons/calendar-days';
	import TagsIcon from '@lucide/svelte/icons/hash';
	import TasksIcon from '@lucide/svelte/icons/square-check';

	import ThemeButton from '$lib/components/ThemeButton.svelte';

	let { onsearch, oncapture }: { onsearch: () => void; oncapture: () => void } = $props();

	let creating = $state(false);
	let draft = $state('');

	const currentId = $derived(page.params.id ?? null);
	const path = $derived(page.url.pathname);

	/**
	 * Journal first, because today is where you land.
	 *
	 * The order is roughly "now, then what to do about it, then everything you
	 * have": the day you are in, the two queues that are bounded work, and then
	 * the whole corpus and its classifications.
	 *
	 * Counts appear only where the number is bounded work, never a backlog.
	 */
	const destinations = $derived([
		{ href: '/journal', label: 'Journal', icon: JournalIcon, count: 0 },
		{ href: '/questions', label: 'Questions', icon: QuestionsIcon, count: session.openQuestions },
		{ href: '/review', label: 'Review', icon: ReviewIcon, count: session.reviewDue },
		{ href: '/library', label: 'Library', icon: LibraryIcon, count: 0 },
		{ href: '/tags', label: 'Tags', icon: TagsIcon, count: 0 },
		{ href: '/tasks', label: 'Tasks', icon: TasksIcon, count: 0 }
	]);

	function isCurrent(href: string) {
		return path.startsWith(href);
	}

	/**
	 * Topics fall asleep on their own. The group holding them is unnumbered,
	 * closed by default, and closes itself again on navigation — a permanently
	 * open, counted list of everything you have neglected is the folder sidebar
	 * in a dimmed hat.
	 */
	let dormantOpen = $state(false);
	let dormant = $state<Topic[]>([]);

	$effect(() => {
		data.boards;
		const awake = new Set(session.topics.map((t) => t.id));
		ipc.allTopics().then((all) => {
			dormant = all.filter((t) => !awake.has(t.id));
		});
	});

	$effect(() => {
		page.url.pathname;
		dormantOpen = false;
	});

	/**
	 * Right-clicking a board.
	 *
	 * Deleting one destroys the arrangement — the frames, the placements, the
	 * pointers — but not the notes: boards import notes rather than owning
	 * them, so every card on it stays in the library. That is what the wording
	 * has to say, because "delete board" sounds like it takes the notes too.
	 */
	function boardMenu(event: MouseEvent, topic: Topic) {
		menu.show(event, [
			{ label: 'Open', onpick: () => goto(`/t/${topic.id}`) },
			{
				label: topic.study ? 'Make it a work board' : 'Make it a study board',
				onpick: () => setMode(topic)
			},
			{
				label: 'Delete board',
				destructive: true,
				onpick: () => removeBoard(topic)
			}
		]);
	}

	async function setMode(topic: Topic) {
		await ipc.setTopicMode(topic.id, !topic.study);
		await session.refresh();
	}

	/** Irreversible, so it asks — and says plainly what survives. */
	async function removeBoard(topic: Topic) {
		const ok = confirm(
			`Delete the board "${topic.title || 'Untitled'}"?

Its arrangement goes: frames, placements and pointers. The notes on it stay in the library.`
		);
		if (!ok) return;

		await ipc.deleteTopic(topic.id);
		await session.refresh();
		if (page.params.id === topic.id) goto(prefs.startPath);
	}

	async function create() {
		const title = draft.trim();
		draft = '';
		creating = false;
		if (!title) return;

		const topic = await session.createTopic(title);
		goto(`/t/${topic.id}`);
	}
</script>

<aside
	class="bg-sidebar text-sidebar-foreground flex h-full min-h-0 flex-col border-r"
	style="border-color: var(--hairline)"
>
	<!-- Runs to the very top of the window, so this is drag region too. -->
	<div data-tauri-drag-region class="flex h-10 shrink-0 items-center px-4 select-none">
		<span class="font-mono text-[10px] tracking-[0.2em] uppercase opacity-40">study</span>
	</div>

	<div class="flex items-center gap-1.5 px-3 pb-3">
		<button
			type="button"
			onclick={onsearch}
			class="bg-background hover:border-foreground/25 flex h-8 grow items-center gap-2 rounded-md border px-2.5 text-left transition-colors"
			style="border-color: var(--hairline)"
		>
			<SearchIcon class="size-3.5 shrink-0 opacity-30" />
			<span class="text-[12px] opacity-40">Search</span>
			<kbd class="ml-auto font-mono text-[9px] opacity-25">CTRL K</kbd>
		</button>

		<button
			type="button"
			onclick={oncapture}
			title="Capture — Ctrl N"
			class="bg-background hover:border-foreground/25 flex size-8 shrink-0 items-center justify-center rounded-md border transition-colors"
			style="border-color: var(--hairline)"
		>
			<PlusIcon class="size-4 opacity-50" />
		</button>
	</div>

	<nav class="flex shrink-0 flex-col gap-px px-3">
		{#each destinations as item (item.href)}
			{@const Icon = item.icon}
			{@const current = isCurrent(item.href)}
			<a
				href={item.href}
				class="flex h-[30px] items-center gap-2.5 rounded-md px-2.5 transition-colors
					{current ? 'bg-sidebar-accent font-medium' : 'hover:bg-sidebar-accent/60'}"
			>
				<Icon class="size-4 shrink-0" style="opacity: {current ? 0.8 : 0.45}" />
				<span class="grow truncate text-[12.5px]">{item.label}</span>
				{#if item.count > 0}
					<!-- Work to do, not debt that accrues. -->
					<span class="font-mono text-[10px] tabular-nums opacity-40">{item.count}</span>
				{/if}
			</a>
		{/each}
	</nav>

	<div class="mx-4 mt-4 border-t" style="border-color: var(--hairline)"></div>

	<div class="flex shrink-0 items-center gap-1 px-4 pt-3 pb-1">
		<span class="label grow">Recent</span>
		<button
			type="button"
			onclick={() => (creating = true)}
			title="New board"
			class="hover:bg-sidebar-accent flex size-5 items-center justify-center rounded opacity-35 transition-all hover:opacity-100"
		>
			<PlusIcon class="size-3.5" />
		</button>
	</div>

	{#if creating}
		<div class="shrink-0 px-3 pb-1" transition:slide={{ duration: 140, easing: cubicOut }}>
			<!-- svelte-ignore a11y_autofocus -->
			<input
				autofocus
				bind:value={draft}
				placeholder="Board name"
				class="bg-background focus:border-foreground h-8 w-full rounded-md border px-2.5 text-[12.5px] outline-none transition-colors"
				style="border-color: var(--hairline)"
				onkeydown={(e) => {
					if (e.key === 'Enter') create();
					if (e.key === 'Escape') {
						creating = false;
						draft = '';
					}
				}}
				onblur={create}
			/>
		</div>
	{/if}

	<nav class="min-h-0 grow overflow-y-auto px-3 pb-2">
		{#each session.topics as topic (topic.id)}
			<a
				animate:flip={{ duration: 180, easing: cubicOut }}
				in:fade={{ duration: 120 }}
				href="/t/{topic.id}"
				oncontextmenu={(e) => boardMenu(e, topic)}
				class="flex h-[30px] items-center gap-2 rounded-md px-2.5 text-[12.5px] transition-colors
					{currentId === topic.id
					? 'bg-sidebar-accent font-medium'
					: 'hover:bg-sidebar-accent/60 opacity-65 hover:opacity-100'}"
			>
				<span class="truncate">{topic.title || 'Untitled'}</span>
				{#if topic.study}
					<span class="label ml-auto shrink-0 !text-[8px]">study</span>
				{/if}
			</a>
		{:else}
			<p class="text-muted-foreground px-2.5 py-4 text-[12px] leading-relaxed">
				Nothing open. Make a board, or capture something and place it later.
			</p>
		{/each}
	</nav>

	<!-- Absent entirely when nothing is sleeping. -->
	{#if dormant.length > 0}
		<div
			class="max-h-52 shrink-0 overflow-y-auto px-3 pb-3"
			transition:slide={{ duration: 160, easing: cubicOut }}
		>
			<button
				type="button"
				onclick={() => (dormantOpen = !dormantOpen)}
				class="hover:bg-sidebar-accent/60 w-full rounded px-2.5 py-1.5 text-left transition-colors"
			>
				<!-- No count. How many boards you have not touched is not something
				     you can act on; it is a number that grows while you live. -->
				<span class="label">{dormantOpen ? 'hide dormant' : 'dormant'}</span>
			</button>

			{#if dormantOpen}
				<div transition:slide={{ duration: 160, easing: cubicOut }}>
					{#each dormant as topic (topic.id)}
						<a
							href="/t/{topic.id}"
							oncontextmenu={(e) => boardMenu(e, topic)}
							class="hover:bg-sidebar-accent/60 flex h-[28px] items-center rounded-md px-2.5 text-[12.5px] opacity-40 transition-all hover:opacity-90"
						>
							<span class="truncate">{topic.title || 'Untitled'}</span>
						</a>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- The only preference with a permanent home. It sits at the bottom because
	     it is the one control here that changes nothing about your work. -->
	<div class="shrink-0 border-t px-3 py-2" style="border-color: var(--hairline)">
		<ThemeButton />
	</div>
</aside>
