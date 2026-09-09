<script lang="ts">
	/**
	 * Loose text on the board.
	 *
	 * The cheapest thing you can put down: no title, no knowledge state, no
	 * card, and no note behind it. It exists so that thinking out loud beside
	 * your cards never begins with deciding that the thought is a note — which
	 * is exactly the decision that stops you writing anything at all.
	 *
	 * It looks like writing on the paper rather than a card *on* the paper: no
	 * fill, no border until you touch it. A loose thought that looked as solid
	 * as a note would be a note, and the difference is the point.
	 *
	 * When it turns out to be a note, the context menu makes it one in place —
	 * same spot, same board, same arrangement around it.
	 */
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';

	type Data = {
		text: string;
		connecting: boolean;
		/** True for the node the text tool just made, so you can type at once. */
		autoedit: boolean;
		onchange: (text: string) => void;
		onabandon: () => void;
		oncontext: (event: MouseEvent) => void;
	};

	let { data, selected }: NodeProps = $props();

	const text = $derived((data as Data).text);
	const connecting = $derived((data as Data).connecting);

	// Placing text and then having to find it and double-click it is two
	// gestures for one thought, which is the cost this node exists to remove.
	//
	// The initial value is deliberate in both cases: these seed local editing
	// state once, and must not be pulled back to the stored text while you type.
	// svelte-ignore state_referenced_locally
	let editing = $state((data as Data).autoedit);
	// svelte-ignore state_referenced_locally
	let draft = $state((data as Data).text);

	function start() {
		draft = text;
		editing = true;
	}

	function commit() {
		editing = false;
		const next = draft.trim();
		if (next === text) return;

		// Empty text is not a thought you had; it is a node you did not fill in.
		// It removes itself rather than lying on the board invisibly.
		if (!next) (data as Data).onabandon();
		else (data as Data).onchange(next);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="h-full w-full rounded-md px-2 py-1.5 transition-colors"
	style="border: 1px solid {selected || editing ? 'var(--hairline-strong)' : 'transparent'}"
	ondblclick={start}
	oncontextmenu={(e) => (data as Data).oncontext(e)}
	role="presentation"
>
	{#if editing}
		<!-- svelte-ignore a11y_autofocus -->
		<textarea
			autofocus
			bind:value={draft}
			onblur={commit}
			onkeydown={(e) => {
				if (e.key === 'Escape') editing = false;
				if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) commit();
			}}
			class="h-full w-full resize-none bg-transparent text-[13px] leading-relaxed outline-none"
		></textarea>
	{:else if text}
		<p class="pointer-events-none text-[13px] leading-relaxed whitespace-pre-wrap">
			{text}
		</p>
	{:else}
		<p class="pointer-events-none text-[13px] leading-relaxed opacity-25">Write…</p>
	{/if}
</div>

<!-- Only reachable while the connect tool is on, so ordinary dragging is never
     hijacked by a handle sitting under the pointer. -->
<Handle type="target" position={Position.Left} class={connecting ? 'canvas-port' : ''} />
<Handle type="source" position={Position.Right} class={connecting ? 'canvas-port' : ''} />
