<script lang="ts">
	/**
	 * The week, across the top of the journal.
	 *
	 * It lives in the top bar rather than the sidebar because it belongs to the
	 * day you are reading: the strip and the note under it are one screen, and
	 * putting the date picker on the far side of the window from the date it
	 * picks was the awkward part of the old layout.
	 *
	 * A calendar in an app that refuses planners needs justifying, so: this is a
	 * *view* onto captures that already exist, not a place to put future work.
	 * Future days are unreachable — a journal you can write forward in is a
	 * planner, and a planner accumulates commitments that grow while you sleep.
	 *
	 * A dot marks a day that has something on it. No counts: how much you wrote
	 * on a Tuesday is not information you can act on.
	 */
	import * as ipc from '$lib/ipc';
	import { data } from '$lib/state/data.svelte';
	import { iso, addDays, FUTURE_DAYS } from '$lib/date';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';

	let {
		day,
		ahead = FUTURE_DAYS,
		onpick
	}: { day: string; ahead?: number; onpick: (day: string) => void } = $props();

	const today = iso(new Date());

	/**
	 * The window ends a few days past the selected day rather than on a Sunday.
	 *
	 * A fixed Monday-to-Sunday week meant that on a Monday five of the seven
	 * cells were unreachable future — most of the strip was dead. Trailing the
	 * selection keeps the reachable past in view and shows just enough of what
	 * is coming to orient you.
	 */
	const anchor = $derived(addDays(new Date(`${day}T00:00:00`), ahead - 6));

	let marked = $state<Set<string>>(new Set());

	const days = $derived(
		Array.from({ length: 7 }, (_, i) => {
			const d = addDays(anchor, i);
			return {
				iso: iso(d),
				label: d.toLocaleDateString(undefined, { weekday: 'short' }).slice(0, 3),
				number: d.getDate()
			};
		})
	);

	$effect(() => {
		// A day gets its dot the moment something lands on it, wherever it was
		// written — this used to need a counter passed down from the page.
		data.notes;
		const from = days[0]?.iso;
		const to = days[6]?.iso;
		if (!from || !to) return;
		ipc.journalMarks(from, to).then((found) => (marked = new Set(found)));
	});

	/** Moves the *selection*, not just the view: the strip has no state of its own. */
	function shift(days: number) {
		const next = iso(addDays(new Date(`${day}T00:00:00`), days));
		onpick(next > today ? today : next);
	}

	const atToday = $derived(day === today);
</script>

<div data-tauri-drag-region class="flex min-w-0 grow items-center gap-0.5">
	<button
		type="button"
		onclick={() => shift(-7)}
		title="Previous week"
		class="hover:bg-accent flex size-7 shrink-0 items-center justify-center rounded-md opacity-40 transition-all hover:opacity-100"
	>
		<ChevronLeft class="size-4" />
	</button>

	<div
		data-tauri-drag-region
		class="scrollbar-none flex min-w-0 grow items-center justify-center gap-0.5 overflow-x-auto"
	>
		{#each days as d (d.iso)}
			{@const future = d.iso > today}
			{@const selected = d.iso === day}
			<button
				type="button"
				disabled={future}
				onclick={() => onpick(d.iso)}
				title={future ? 'Not yet' : d.iso}
				class="flex w-[52px] shrink-0 flex-col items-center rounded-md py-1 transition-colors
					{future ? 'cursor-default opacity-20' : selected ? '' : 'hover:bg-accent'}"
				style={selected ? 'background: var(--accent)' : ''}
			>
				<span
					class="font-mono text-[9px] tracking-wider uppercase"
					style="opacity: {selected ? 0.7 : 0.4}"
				>
					{d.label}
				</span>
				<span class="text-[12.5px] tabular-nums {selected ? 'font-semibold' : ''}">
					{d.number}
				</span>
				<!-- Presence, not volume. -->
				<span
					class="mt-px size-[3px] rounded-full"
					style="background: currentColor; opacity: {marked.has(d.iso) && !future ? 0.5 : 0}"
				></span>
			</button>
		{/each}
	</div>

	<button
		type="button"
		onclick={() => shift(7)}
		disabled={day === today}
		title="Next week"
		class="hover:bg-accent flex size-7 shrink-0 items-center justify-center rounded-md opacity-40 transition-all hover:opacity-100 disabled:opacity-10"
	>
		<ChevronRight class="size-4" />
	</button>

	<!-- Absent when it would do nothing. -->
	{#if !atToday}
		<button
			type="button"
			onclick={() => onpick(today)}
			class="hover:bg-accent ml-1 shrink-0 rounded-md border px-2 py-1 text-[11.5px] transition-colors"
			style="border-color: var(--hairline)"
		>
			Today
		</button>
	{/if}
</div>
