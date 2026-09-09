<script lang="ts">
	/**
	 * One panel, six tools, chosen by the icon row at its top.
	 *
	 * It inspects what the centre is showing — the screens you *navigate to*
	 * live in the middle of the window.
	 *
	 * **Closed means gone.** Not a rail of icons down the edge: a rail costs
	 * width and gives nothing back, and the space belongs to the centre. It
	 * slides out, and the button in the top bar brings it back.
	 *
	 * Every tool stays pressable, everywhere. A tool you cannot press is a tool
	 * you cannot find out about, so pressing Info with no note open says what
	 * Info would show you — which is how you learn what it is for.
	 *
	 * The row scrolls sideways when the panel is narrow, and fades at the edge
	 * so it is visible that there is more.
	 */
	import { slide } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
	import { slot, SLOT_LABEL, type SlotContext, type SlotTool } from '$lib/state/slot.svelte';
	import { prefs } from '$lib/state/prefs.svelte';
	import type { Frame, Topic } from '$lib/types';
	import LibraryTool from './LibraryTool.svelte';
	import InfoTool from './InfoTool.svelte';
	import QuestionsTool from './QuestionsTool.svelte';
	import OutlineTool from './OutlineTool.svelte';
	import TasksTool from './TasksTool.svelte';
	import SourcePane from '$lib/components/memory/SourcePane.svelte';

	import LibraryIcon from '@lucide/svelte/icons/layout-grid';
	import InfoIcon from '@lucide/svelte/icons/info';
	import SourceIcon from '@lucide/svelte/icons/book-open-text';
	import QuestionsIcon from '@lucide/svelte/icons/circle-help';
	import OutlineIcon from '@lucide/svelte/icons/list-tree';
	import TasksIcon from '@lucide/svelte/icons/square-check';
	import PanelIcon from '@lucide/svelte/icons/panel-right-close';

	let {
		context,
		topic = null,
		frames = [],
		sourceNoteId = null,
		onfocusframe
	}: {
		context: SlotContext;
		topic?: Topic | null;
		frames?: Frame[];
		sourceNoteId?: string | null;
		onfocusframe?: (frameId: string) => void;
	} = $props();

	const icons: Record<SlotTool, typeof InfoIcon> = {
		library: LibraryIcon,
		info: InfoIcon,
		source: SourceIcon,
		questions: QuestionsIcon,
		tasks: TasksIcon,
		outline: OutlineIcon
	};

	const active = $derived(slot.tool);
	/** Whether the chosen tool can say anything here. Never gates the button. */
	const speaks = $derived(slot.applies(active, context));

	/**
	 * The tool row, when it is narrower than its tools.
	 *
	 * At the panel's minimum width the row always overflows — the window
	 * controls take a fixed bite out of the header — so the last tools were
	 * simply unreachable. A vertical wheel is redirected sideways, and the row
	 * can be dragged, because horizontal scrolling is a trackpad gesture and
	 * plenty of mice do not have one.
	 */
	let row = $state<HTMLDivElement | null>(null);

	function onwheel(event: WheelEvent) {
		if (!row || row.scrollWidth <= row.clientWidth) return;
		// Only claim the gesture when there is somewhere to go.
		event.preventDefault();
		row.scrollLeft += event.deltaY + event.deltaX;
	}

	function ondown(event: PointerEvent) {
		if (!row || row.scrollWidth <= row.clientWidth) return;

		const startX = event.clientX;
		const startLeft = row.scrollLeft;
		let dragged = false;

		const move = (e: PointerEvent) => {
			const dx = e.clientX - startX;
			// A few pixels of slop, so a click on a tool is still a click.
			if (!dragged && Math.abs(dx) < 4) return;
			dragged = true;
			if (row) row.scrollLeft = startLeft - dx;
		};

		const up = () => {
			window.removeEventListener('pointermove', move);
			window.removeEventListener('pointerup', up);
		};

		window.addEventListener('pointermove', move);
		window.addEventListener('pointerup', up);
	}
</script>

{#if slot.open}
	<aside
		transition:slide={{ axis: 'x', duration: 190, easing: cubicOut }}
		class="bg-background flex h-full min-h-0 shrink-0 flex-col border-l"
		style="width: {prefs.slotWidth}px; border-color: var(--hairline)"
	>
		<!-- Runs to the top of the window, so this row is drag region too. The
		     tools are kept clear of the window controls by the spacer at the end. -->
		<header
			data-tauri-drag-region
			class="flex h-10 shrink-0 items-center border-b select-none"
			style="border-color: var(--hairline)"
		>
			<!-- Scrolls sideways when the panel is narrow. The fade at the trailing
			     edge is what says there is more than fits, and the wheel and a drag
			     both move it, because a trackpad's horizontal gesture is not
			     something every mouse has. -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:this={row}
				{onwheel}
				onpointerdown={ondown}
				class="scrollbar-none flex min-w-0 grow items-center gap-0.5 overflow-x-auto px-1.5"
				style="mask-image: linear-gradient(to right, #000 calc(100% - 14px), transparent)"
			>
				{#each slot.tools as tool (tool)}
					{@const Icon = icons[tool]}
					<button
						type="button"
						onclick={() => slot.show(tool)}
						title={SLOT_LABEL[tool]}
						class="flex size-7 shrink-0 items-center justify-center rounded-md transition-colors
							{active === tool ? 'bg-accent' : 'hover:bg-accent/60 opacity-45'}"
					>
						<Icon class="size-4" />
					</button>
				{/each}
			</div>

			<!-- Pinned outside the scroller: closing the panel is how you get the
			     space back, so it must never be the thing that scrolled away. -->
			<button
				type="button"
				onclick={() => slot.toggle()}
				title="Hide panel"
				class="hover:bg-accent mr-1 flex size-7 shrink-0 items-center justify-center rounded-md opacity-35 transition-all hover:opacity-100"
			>
				<PanelIcon class="size-4" />
			</button>

			<!-- Kept clear of the window controls, and outside the scroller so it
			     cannot scroll away and let the tools slide underneath them. -->
			<div data-tauri-drag-region class="h-full w-[136px] shrink-0"></div>
		</header>

		<div class="flex min-h-0 grow flex-col">
			{#if !speaks}
				<!-- The tool is live, it just has nothing in front of it. Saying what
				     it would show is how you find out what the tool is for. -->
				<div class="px-4 py-4">
					<p class="label mb-1.5">{SLOT_LABEL[active]}</p>
					<p class="text-muted-foreground text-[12px] leading-relaxed">
						{slot.needs(active)}
					</p>
				</div>
			{:else if active === 'library'}
				<LibraryTool topicId={topic?.id ?? null} />
			{:else if active === 'info' && slot.noteId}
				<InfoTool noteId={slot.noteId} onopen={slot.openNote} />
			{:else if active === 'source'}
				<SourcePane {sourceNoteId} onpick={slot.requestSourcePick} />
			{:else if active === 'questions'}
				<QuestionsTool topicId={topic?.id ?? null} />
			{:else if active === 'tasks'}
				<TasksTool day={slot.day} />
			{:else if active === 'outline'}
				<OutlineTool {frames} study={topic?.study ?? false} onfocus={onfocusframe} />
			{/if}
		</div>
	</aside>
{/if}
