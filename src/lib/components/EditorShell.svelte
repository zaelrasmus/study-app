<svelte:window {onkeydown} />

<script lang="ts">
	/**
	 * The container the editor lives in.
	 *
	 * A right-hand sheet was too narrow: memory mode is a split view, and the
	 * source and the writing surface were fighting over the same few hundred
	 * pixels. Both presentations here are wide enough for that split.
	 *
	 * `full` is an absolutely positioned child of the working area rather than a
	 * viewport overlay — `<main>` is the nearest positioned ancestor, so
	 * `inset-0` lands exactly between the sidebar and the window edge without
	 * hard-coding either dimension.
	 */
	import * as Dialog from '$lib/components/ui/dialog';
	import { prefs, type EditorPresentation } from '$lib/state/prefs.svelte';
	import type { Snippet } from 'svelte';
	import MaximiseIcon from '@lucide/svelte/icons/maximize-2';
	import MinimiseIcon from '@lucide/svelte/icons/minimize-2';
	import XIcon from '@lucide/svelte/icons/x';

	let {
		onclose,
		children
	}: { onclose: () => void; children: Snippet<[{ presentationToggle: Snippet }]> } = $props();

	const presentation = $derived(prefs.editorPresentation);

	function toggle() {
		const next: EditorPresentation = presentation === 'full' ? 'dialog' : 'full';
		prefs.setEditorPresentation(next);
	}

	function onkeydown(event: KeyboardEvent) {
		// The dialog presentation closes itself; only the full one needs this.
		if (presentation !== 'full') return;
		if (event.key !== 'Escape') return;
		// Let menus, pickers and the slash command close themselves first.
		if (document.querySelector('[data-slot="dialog-content"], [role="listbox"]')) return;
		onclose();
	}
</script>

{#snippet presentationToggle()}
	<button
		type="button"
		onclick={toggle}
		title={presentation === 'full' ? 'Show as a dialog' : 'Fill the working area'}
		class="hover:bg-accent flex size-6 shrink-0 items-center justify-center rounded opacity-45 transition-all hover:opacity-100"
	>
		{#if presentation === 'full'}
			<MinimiseIcon class="size-3.5" />
		{:else}
			<MaximiseIcon class="size-3.5" />
		{/if}
	</button>
{/snippet}

{#snippet closeButton()}
	<button
		type="button"
		onclick={onclose}
		title="Close — Esc"
		class="hover:bg-accent flex size-6 shrink-0 items-center justify-center rounded opacity-45 transition-all hover:opacity-100"
	>
		<XIcon class="size-3.5" />
	</button>
{/snippet}

{#if presentation === 'full'}
	<section
		class="bg-background absolute inset-0 z-40 flex flex-col"
		aria-label="Note editor"
	>
		{@render children({ presentationToggle: chrome })}
	</section>
{:else}
	<Dialog.Root open onOpenChange={(v) => !v && onclose()}>
		<Dialog.Content
			showCloseButton={false}
			class="flex h-[86vh] max-h-none w-[min(1100px,94vw)] max-w-none flex-col gap-0 p-0"
		>
			<Dialog.Header class="sr-only">
				<Dialog.Title>Note editor</Dialog.Title>
			</Dialog.Header>
			{@render children({ presentationToggle: chrome })}
		</Dialog.Content>
	</Dialog.Root>
{/if}

{#snippet chrome()}
	{@render presentationToggle()}
	{@render closeButton()}
{/snippet}
