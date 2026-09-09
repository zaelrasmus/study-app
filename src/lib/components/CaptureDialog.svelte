<script lang="ts">
	/**
	 * Capture.
	 *
	 * One destination. The app does not read your text to guess what it is —
	 * the old rule routed anything ending in `?` to the question queue, which is
	 * wrong for Spanish, where nobody types `¿` while capturing messily. So
	 * "Que es una CPU" is a question that would have been filed as a note.
	 *
	 * Type is a keystroke, not an inference: Enter saves a note, Ctrl+Enter
	 * saves a question. Both are one key away and neither asks anything else.
	 *
	 * A question does pick up **where you were**. Doubts arrive while you are
	 * writing messily about something, so a question caught with a board open is
	 * about that board and is attached to it — otherwise it lands in the global
	 * queue and the board's own panel shows nothing, which is exactly what it
	 * used to do.
	 */
	import * as Dialog from '$lib/components/ui/dialog';
	import { session } from '$lib/state/session.svelte';
	import * as ipc from '$lib/ipc';

	let {
		open = $bindable(false),
		topicId = null,
		sourceNoteId = null,
		topicTitle = null
	}: {
		open?: boolean;
		/** The board in view. A question caught here is about it. */
		topicId?: string | null;
		/** The note being written. What the doubt came out of. */
		sourceNoteId?: string | null;
		topicTitle?: string | null;
	} = $props();

	let text = $state('');
	let lastSaved = $state<'note' | 'question' | null>(null);

	async function commit(asQuestion: boolean) {
		const value = text.trim();
		if (!value) return;

		if (asQuestion) {
			// Captures are the lowest-priority origin, and the queue says so.
			await ipc.createQuestion(value, 'capture', topicId, sourceNoteId);
			session.openQuestions += 1;
			lastSaved = 'question';
		} else {
			await session.capture(value);
			lastSaved = 'note';
		}

		text = '';
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'Enter' || event.shiftKey) return;
		event.preventDefault();
		commit(event.ctrlKey || event.metaKey);
	}

	$effect(() => {
		if (!open) {
			text = '';
			lastSaved = null;
		}
	});
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="gap-0 p-0 sm:max-w-xl" showCloseButton={false}>
		<Dialog.Header class="sr-only">
			<Dialog.Title>Capture</Dialog.Title>
			<Dialog.Description>
				Enter saves a note. Ctrl and Enter saves a question.
			</Dialog.Description>
		</Dialog.Header>

		<!-- svelte-ignore a11y_autofocus -->
		<textarea
			autofocus
			bind:value={text}
			{onkeydown}
			rows="3"
			placeholder="Anything at all."
			class="w-full resize-none bg-transparent px-4 py-4 text-[15px] leading-relaxed outline-none"
		></textarea>

		<div
			class="text-muted-foreground flex items-center gap-4 border-t px-4 py-2 font-mono text-[10px]"
			style="border-color: var(--hairline)"
		>
			<span><kbd>Enter</kbd> note</span>
			<span>
				<kbd>Ctrl Enter</kbd> question{#if topicTitle}
					<span class="opacity-60"> → {topicTitle}</span>
				{/if}
			</span>
			<span class="ml-auto"><kbd>Esc</kbd> close</span>
		</div>

		{#if lastSaved}
			<p
				class="text-muted-foreground border-t px-4 py-2 text-[11px]"
				style="border-color: var(--hairline)"
			>
				Saved as a {lastSaved}. Keep going.
			</p>
		{/if}
	</Dialog.Content>
</Dialog.Root>
