<script lang="ts">
	/**
	 * Distillation as a drag.
	 *
	 * Select the blocks that belong to one concept, then pull them onto the
	 * canvas. That is the fast lane: it organises material with the source in
	 * front of you, which is exactly why the result is a draft and never
	 * `recalled`. Memory mode remains the only route to recalled, so no rule
	 * bends to make this convenient.
	 *
	 * The blocks are copied. The long note keeps them, so pulling a concept out
	 * can never quietly damage the thing you pulled it from.
	 */
	import { drag } from '$lib/state/drag.svelte';
	import type { Editor } from '$lib/components/edra/shadcn/index.js';
	import { DOMSerializer } from '@tiptap/pm/model';
	import GripIcon from '@lucide/svelte/icons/grip-vertical';

	let { editor, disabled = false }: { editor: Editor; disabled?: boolean } = $props();

	let selection = $state({ empty: true, text: '' });

	$effect(() => {
		const update = () => {
			const { from, to, empty } = editor.state.selection;
			selection = {
				empty,
				text: empty ? '' : editor.state.doc.textBetween(from, to, ' ').trim()
			};
		};

		editor.on('selectionUpdate', update);
		editor.on('transaction', update);
		update();

		return () => {
			editor.off('selectionUpdate', update);
			editor.off('transaction', update);
		};
	});

	function grab(event: PointerEvent) {
		if (selection.empty || disabled) return;

		const { from, to } = editor.state.selection;
		const slice = editor.state.doc.slice(from, to);

		// Serialise the selection on its own so the new note is a real document
		// rather than a string, and keeps its formatting.
		const json = { type: 'doc', content: slice.content.toJSON() ?? [] };

		// HTML from the same schema, so the assembled document renders the
		// extract exactly as it read in the note it came from.
		const fragment = DOMSerializer.fromSchema(editor.schema).serializeFragment(slice.content);
		const holder = document.createElement('div');
		holder.appendChild(fragment);

		drag.begin(
			{
				kind: 'blocks',
				json: JSON.stringify(json),
				html: holder.innerHTML,
				text: selection.text,
				label: selection.text.slice(0, 60)
			},
			event
		);
	}
</script>

{#if !selection.empty && !disabled}
	<button
		type="button"
		onpointerdown={grab}
		title="Drag onto the canvas to distil it into a note"
		class="bg-card hover:bg-accent flex shrink-0 cursor-grab touch-none items-center gap-1.5 rounded border px-2 py-1 text-[10.5px] transition-colors active:cursor-grabbing"
		style="border-color: var(--hairline-strong)"
	>
		<GripIcon class="size-3 opacity-40" />
		Distil selection
	</button>
{/if}
