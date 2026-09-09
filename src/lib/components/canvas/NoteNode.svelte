<script lang="ts">
	/**
	 * A note on the canvas.
	 *
	 * On a **work board** the card carries no epistemic layer at all: no state
	 * word, no frame word, no colour. It is a note you wanted to write, and the
	 * app has nothing to say about how far it is from anywhere.
	 *
	 * On a **study board** it carries one footer line — and even there, only if
	 * the note has actually attempted memory. `raw` and `draft` render nothing,
	 * because two stacked negatives ("not distilled · not in document") tell you
	 * you are failing at two things you were not attempting.
	 *
	 * Kind and state are never shown together. They are different axes and
	 * putting both on one card reads as two verdicts on the same thing.
	 */
	import { Handle, NodeResizer, Position, type NodeProps } from '@xyflow/svelte';
	import StateMark from './StateMark.svelte';
	import type { Note } from '$lib/types';
	import { cardFace, STATE_LABEL, STATE_MEANING, STATE_NEEDS_ACTION } from '$lib/types';
	import { fill } from '$lib/cardColour';

	type Data = {
		note: Note;
		framed: boolean;
		study: boolean;
		/** A chosen colour, per placement. Not a signal. */
		colour: string | null;
		/** True while the connect tool is on, which is the only time ports show. */
		connecting: boolean;
		onopen: (noteId: string) => void;
		oncontext: (event: MouseEvent) => void;
	};

	let { data, selected }: NodeProps = $props();

	const note = $derived((data as Data).note);
	const framed = $derived((data as Data).framed);
	const study = $derived((data as Data).study);
	const paper = $derived(fill((data as Data).colour));
	const connecting = $derived((data as Data).connecting);

	/** The epistemic layer appears only on a study board, and only once the note
	 *  has something to say about memory. */
	const showState = $derived(study && note.visible);

	const face = $derived(cardFace(note));
	/** Only shown when it is not already the face, so nothing is said twice. */
	const preview = $derived(
		note.body_text.trim() === face ? '' : note.body_text.trim()
	);

	const accent = $derived(
		!showState
			? null
			: note.state === 'recalled'
				? 'var(--state-recalled)'
				: STATE_NEEDS_ACTION[note.state]
					? 'var(--state-action)'
					: null
	);

	const border = $derived(
		selected ? 'var(--foreground)' : (accent ?? 'var(--hairline-strong)')
	);

	/**
	 * The state tint only appears on an uncoloured card. Once you have chosen a
	 * colour, that colour is the card's surface and state falls back to the rule
	 * and the mark alone — otherwise two washes fight and neither reads.
	 */
	const tint = $derived(
		paper
			? 'transparent'
			: accent === 'var(--state-recalled)'
				? 'var(--state-recalled-soft)'
				: accent
					? 'var(--state-action-soft)'
					: 'transparent'
	);
</script>

<NodeResizer
	isVisible={selected}
	minWidth={170}
	minHeight={80}
	lineClass="!border-transparent"
	handleClass="!size-1.5 !rounded-none !border-foreground/40 !bg-background"
/>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	class="relative flex h-full w-full cursor-default flex-col gap-1 overflow-hidden rounded-md py-2.5 pr-3 pl-3 transition-all"
	style="background: {paper ?? 'var(--card)'};
		border: 1px solid {paper && !accent && !selected ? 'var(--card-edge)' : border};
		box-shadow: {selected ? '0 1px 12px -4px var(--hairline-strong)' : 'none'}"
	title={showState ? STATE_MEANING[note.state] : undefined}
	ondblclick={() => (data as Data).onopen(note.id)}
	oncontextmenu={(e) => (data as Data).oncontext(e)}
>
	{#if accent}
		<!-- The state rule down the leading edge: readable across a whole canvas
		     without reading a word. -->
		<span class="absolute top-0 bottom-0 left-0 w-[2px]" style="background: {accent}"></span>
		<span class="pointer-events-none absolute inset-0" style="background: {tint}"></span>
	{/if}

	<header class="relative flex items-start gap-2">
		{#if showState}
			<StateMark
				state={note.state}
				visible={note.visible}
				neverContrasted={note.never_contrasted}
				class="mt-[2px]"
			/>
		{/if}
		<h3 class="min-w-0 grow text-[12.5px] leading-snug font-medium tracking-tight">
			{face || 'Untitled'}
		</h3>
	</header>

	{#if preview}
		<p class="text-muted-foreground relative line-clamp-3 text-[11px] leading-[1.55]">
			{preview}
		</p>
	{/if}

	{#if showState}
		<!-- One line, and the absent states are phrased neutrally: "on canvas",
		     never "not in document". -->
		<footer class="label relative mt-auto flex items-center gap-1.5 !text-[8px]">
			<span style="color: {accent}">{STATE_LABEL[note.state]}</span>
			<span class="opacity-30">·</span>
			<span class="opacity-70">{framed ? 'in document' : 'on canvas'}</span>
		</footer>
	{/if}
</article>

<!-- Only reachable while the connect tool is on, so ordinary dragging is never
     hijacked by a handle sitting under the pointer. -->
<Handle type="target" position={Position.Left} class={connecting ? 'canvas-port' : ''} />
<Handle type="source" position={Position.Right} class={connecting ? 'canvas-port' : ''} />
