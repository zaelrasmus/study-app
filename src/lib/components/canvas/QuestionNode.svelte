<script lang="ts">
	/**
	 * An open question, sitting on the board.
	 *
	 * Doubts arrive while you are writing messily about something, which means
	 * the thing they are about is usually already arranged in front of you. A
	 * question that can only live in a queue makes you hold the connection in
	 * your head; one you can put down next to the three cards that might answer
	 * it does not.
	 *
	 * It is drawn as a question, not as a note: no fill, a dashed edge, and the
	 * text in the serif the queue uses. An open question is not a thing you
	 * have — it is a hole in what you have — and it should not look like a card
	 * you finished.
	 */
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import { ORIGIN_TAG, type Question } from '$lib/types';
	import HelpIcon from '@lucide/svelte/icons/circle-help';

	type Data = {
		question: Question;
		connecting: boolean;
		onanswer: (question: Question) => void;
		oncontext: (event: MouseEvent) => void;
	};

	let { data, selected }: NodeProps = $props();

	const question = $derived((data as Data).question);
	const connecting = $derived((data as Data).connecting);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	class="flex h-full w-full cursor-default flex-col gap-2 rounded-md px-3 py-2.5 transition-all"
	style="border: 1px dashed {selected ? 'var(--foreground)' : 'var(--hairline-strong)'};
		background: color-mix(in oklch, var(--card) 60%, transparent)"
	title="Double-click to answer it with a new note"
	ondblclick={() => (data as Data).onanswer(question)}
	oncontextmenu={(e) => (data as Data).oncontext(e)}
>
	<div class="flex items-start gap-2">
		<HelpIcon class="mt-[2px] size-3.5 shrink-0 opacity-30" />
		<p class="min-w-0 grow font-serif text-[13px] leading-snug">{question.text}</p>
	</div>

	<span class="label mt-auto !text-[8px]">{ORIGIN_TAG[question.origin]}</span>
</article>

<!-- Only reachable while the connect tool is on, so ordinary dragging is never
     hijacked by a handle sitting under the pointer. -->
<Handle type="target" position={Position.Left} class={connecting ? 'canvas-port' : ''} />
<Handle type="source" position={Position.Right} class={connecting ? 'canvas-port' : ''} />
