<script lang="ts">
	/**
	 * Canvas chrome.
	 *
	 * Tools carry words and keys, not just glyphs — an icon-only toolbar is the
	 * fastest way to make a canvas feel unknowable. The active tool always states
	 * what it does in the strip along the bottom, so there is never a mode you
	 * are in without being told what it means.
	 */
	import MousePointerIcon from '@lucide/svelte/icons/mouse-pointer-2';
	import PenIcon from '@lucide/svelte/icons/pen-line';
	import FrameIcon from '@lucide/svelte/icons/square-dashed';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import HelpIcon from '@lucide/svelte/icons/circle-help';
	import BookIcon from '@lucide/svelte/icons/book-open';
	import PenToolIcon from '@lucide/svelte/icons/pen-tool';
	import * as Popover from '$lib/components/ui/popover';
	import StateMark from './StateMark.svelte';
	import TypeIcon from '@lucide/svelte/icons/type';
	import ArrowIcon from '@lucide/svelte/icons/spline';
	import { TOOL_HINT, type Tool } from './tools';

	let {
		tool = $bindable('select'),
		study,
		onmode,
		ondocument,
		onrecall
	}: {
		tool?: Tool;
		/** Study board or work board. Presentation only. */
		study: boolean;
		onmode: (study: boolean) => void;
		ondocument: () => void;
		onrecall: () => void;
	} = $props();

	const tools: { id: Tool; label: string; key: string; icon: typeof PenIcon }[] = [
		{ id: 'select', label: 'Move', key: 'V', icon: MousePointerIcon },
		{ id: 'draw', label: 'Draw', key: 'P', icon: PenIcon },
		{ id: 'frame', label: 'Frame', key: 'F', icon: FrameIcon },
		{ id: 'text', label: 'Text', key: 'T', icon: TypeIcon },
		{ id: 'connect', label: 'Point', key: 'C', icon: ArrowIcon }
	];
</script>

<!-- Tools, top-left. -->
<div
	class="bg-card/95 absolute top-3 left-3 z-20 flex items-center gap-px rounded-md border p-1 shadow-sm backdrop-blur"
	style="border-color: var(--hairline)"
>
	{#each tools as t (t.id)}
		{@const Icon = t.icon}
		<button
			type="button"
			onclick={() => (tool = t.id)}
			class="flex items-center gap-1.5 rounded px-2 py-1.5 transition-colors
				{tool === t.id ? 'bg-foreground text-background' : 'hover:bg-accent'}"
		>
			<Icon class="size-3.5" />
			<span class="text-[11px] font-medium">{t.label}</span>
			<kbd
				class="font-mono text-[9px] {tool === t.id ? 'opacity-50' : 'opacity-35'}"
			>{t.key}</kbd>
		</button>
	{/each}
</div>

<!-- Outputs, top-right. These are the three things you can do *with* an
     arrangement once it exists. -->
<div
	class="bg-card/95 absolute top-3 right-3 z-20 flex items-center gap-px rounded-md border p-1 shadow-sm backdrop-blur"
	style="border-color: var(--hairline)"
>
	{#if study}
	<button
		type="button"
		onclick={ondocument}
		class="hover:bg-accent flex items-center gap-1.5 rounded px-2 py-1.5 transition-colors"
	>
		<FileTextIcon class="size-3.5" />
		<span class="text-[11px] font-medium">Document</span>
	</button>

	<button
		type="button"
		onclick={onrecall}
		title="Reconstruct this topic from memory"
		class="hover:bg-accent flex items-center gap-1.5 rounded px-2 py-1.5 transition-colors"
	>
		<BrainIcon class="size-3.5" />
		<span class="text-[11px] font-medium">Recall</span>
	</button>
	{/if}

	<!-- One toggle, non-destructive both ways. Study is the mode you opt into
	     for something you are trying to learn, not the mode you opt out of every
	     time you want to think. -->
	<button
		type="button"
		onclick={() => onmode(!study)}
		title={study
			? 'Study board — the memory layer is visible. Switch to a work board.'
			: 'Work board. Switch to a study board to show the memory layer.'}
		class="hover:bg-accent flex items-center gap-1.5 rounded px-2 py-1.5 transition-colors"
	>
		{#if study}
			<BookIcon class="size-3.5" />
			<span class="text-[11px] font-medium">Study</span>
		{:else}
			<PenToolIcon class="size-3.5" />
			<span class="text-[11px] font-medium">Work</span>
		{/if}
	</button>
</div>

<!-- The always-on hint line. Bottom-left, quiet, and never a modal. -->
<div class="absolute bottom-3 left-3 z-20 flex items-center gap-2">
	<Popover.Root>
		<Popover.Trigger
			class="bg-card/95 hover:bg-accent flex size-7 items-center justify-center rounded-md border shadow-sm backdrop-blur transition-colors"
			title="What the marks mean"
		>
			<HelpIcon class="size-3.5 opacity-60" />
		</Popover.Trigger>
		<Popover.Content side="top" align="start" class="w-80 p-0">
			<div class="border-b px-3 py-2">
				<p class="label">Gestures</p>
			</div>
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 px-3 py-2.5 text-[11px]">
				<dt class="font-mono opacity-50">dbl-click</dt>
				<dd>New note where you clicked</dd>
				<dt class="font-mono opacity-50">dbl-click</dt>
				<dd>On a note — open the editor</dd>
				<dt class="font-mono opacity-50">drag</dt>
				<dd>A note into a frame commits it to the document</dd>
				<dt class="font-mono opacity-50">F</dt>
				<dd>Drag out a frame — a section of the document</dd>
				<dt class="font-mono opacity-50">P</dt>
				<dd>Draw freehand</dd>
				<dt class="font-mono opacity-50">Ctrl N</dt>
				<dd>Capture anything, from anywhere</dd>
			</dl>

			<div class="border-t border-b px-3 py-2">
				<p class="label">Marks</p>
			</div>
			<dl class="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2 px-3 py-2.5 text-[11px]">
				<dt><StateMark state="raw" /></dt>
				<dd>Captured, not written into</dd>
				<dt><StateMark state="draft" /></dt>
				<dd>Written with the source visible</dd>
				<dt><StateMark state="recalled" /></dt>
				<dd>Reproduced from memory</dd>
				<dt><StateMark state="edited_since_recall" /></dt>
				<dd>Edited since you reproduced it</dd>
			</dl>
		</Popover.Content>
	</Popover.Root>

	<p
		class="bg-card/95 rounded-md border px-2.5 py-1.5 text-[11px] shadow-sm backdrop-blur"
		style="border-color: var(--hairline)"
	>
		{TOOL_HINT[tool]}
	</p>
</div>
