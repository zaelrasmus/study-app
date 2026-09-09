<script lang="ts">
	/**
	 * The note editor, in two configurations of one surface.
	 *
	 * **Memory mode off** — split view. The source sits beside the editor, and
	 * the action that writes `last_recall_at` is visibly present but disabled.
	 * Hovering it says why: with the source in front of you, this is
	 * reorganising, and the note stops at draft.
	 *
	 * **Memory mode on** — single column. The source pane is *removed from the
	 * DOM*, not dimmed and not blurred, because if it can be looked at it will
	 * be. The editor is cleared and expands into the space: you are writing from
	 * memory, not editing what is already there.
	 *
	 * The four steps of the flow, and which fact each one writes:
	 *   1. Write blind.                      (nothing)
	 *   2. Save.                             last_recall_at, content_hash_at_recall
	 *   3. The source is revealed.           (nothing)
	 *   4. Mark: matches, or I left things out.  last_checked_at, + Questions
	 *
	 * Steps 2 and 4 are separate on purpose. Closing the editor after step 2 is
	 * allowed, and leaves the note recalled-but-never-contrasted — which is a
	 * real epistemic state, not an error.
	 */
	import { onDestroy, onMount } from 'svelte';
	import EditorShell from '$lib/components/EditorShell.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { Button } from '$lib/components/ui/button';
	import { Edra, createEditor } from '$lib/components/edra/shadcn/index.js';
	import * as ipc from '$lib/ipc';
	import { slot } from '$lib/state/slot.svelte';
	import type { Note } from '$lib/types';
	import { STATE_MEANING } from '$lib/types';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	import ContrastStep from '$lib/components/memory/ContrastStep.svelte';
	import NotePicker from '$lib/components/memory/NotePicker.svelte';
	import DistilHandle from '$lib/components/memory/DistilHandle.svelte';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import EyeIcon from '@lucide/svelte/icons/eye';

	let {
		noteId,
		topicId = null,
		onclose,
		onsaved,
		ondistil
	}: {
		noteId: string;
		topicId?: string | null;
		onclose: () => void;
		/** Optional: some screens re-read their own list on close instead. */
		onsaved?: (note: Note) => void;
		/** Tells the canvas which note a distil-drag is coming out of. */
		ondistil?: (noteId: string) => void;
	} = $props();

	type Mode = 'off' | 'writing' | 'contrast';

	let note = $state<Note | null>(null);
	let title = $state('');
	/** One line describing the concept. What the canvas and library render. */
	let summary = $state('');
	let dirty = $state(false);
	let ready = $state(false);
	let mode = $state<Mode>('off');
	let pickerOpen = $state(false);
	/** Distinguishes picking a source from picking a note to mention. */
	let picking = $state<'source' | 'mention'>('source');

	/**
	 * Set when memory mode is switched off part-way through writing. The recall
	 * action stays disabled until the editor is reopened: once the source has been
	 * back on screen, this attempt cannot honestly count.
	 */
	let disqualified = $state(false);

	/** The body as it was before the blind rewrite. Held for the Contrast step. */
	let outgoing = $state<{ title: string; html: string; text: string } | null>(null);

	/** The consequence being shown before it happens, never after. */
	let confirming = $state<'enter' | 'exit' | null>(null);

	/** Set when a recall landed inside the cooling window. */
	let coolingNotice = $state(false);

	let timer: ReturnType<typeof setTimeout> | undefined;

	const editor = createEditor({ onUpdate: () => schedule() });

	onMount(async () => {
		const loaded = await ipc.getNote(noteId);
		if (!loaded) return;

		note = loaded;
		title = loaded.title;
		summary = loaded.summary;
		loadBody(loaded.body_json);
		slot.enterNote(loaded.id, loaded.source_note_id, () => {
					picking = 'source';
					pickerOpen = true;
				});
		ondistil?.(loaded.id);
		ready = true;
	});

	function loadBody(json: string) {
		if (!editor || !json) return;
		try {
			editor.commands.setContent(JSON.parse(json));
		} catch {
			// A body we cannot parse is left alone rather than silently wiped.
		}
	}

	// -- ordinary editing -------------------------------------------------

	/** Debounced. Only runs outside memory mode: a blind rewrite is saved by an
	 *  explicit act, never by an autosave that would record a half-written recall. */
	function schedule() {
		if (!ready || mode !== 'off') return;
		dirty = true;
		clearTimeout(timer);
		timer = setTimeout(save, 700);
	}

	async function save() {
		if (!note || !editor || mode !== 'off') return;

		const updated = await ipc.saveNote(
			note.id,
			title,
			summary,
			JSON.stringify(editor.getJSON()),
			editor.getHTML(),
			editor.getText()
		);

		note = updated;
		dirty = false;
		onsaved?.(updated);
	}

	// -- memory mode ------------------------------------------------------

	function enterMemoryMode() {
		if (!editor || !note) return;

		// Snapshot before clearing: this is what comes back at Contrast.
		const html = editor.getHTML();
		const text = editor.getText();
		outgoing = text.trim() || note.title ? { title: note.title, html, text } : null;

		editor.commands.clearContent();
		// The source must be unreachable, not merely un-chosen.
		slot.lock();
		confirming = null;
		coolingNotice = false;
		mode = 'writing';
	}

	function leaveMemoryMode() {
		slot.unlock();
		confirming = null;
		// What you wrote stays; only the claim is withdrawn.
		disqualified = true;
		mode = 'off';
	}

	/** Step 2. */
	async function saveFromMemory() {
		if (!note || !editor) return;

		const updated = await ipc.recordRecall(
			note.id,
			JSON.stringify(editor.getJSON()),
			editor.getHTML(),
			editor.getText()
		);

		note = updated;
		onsaved?.(updated);

		// The cooling rule is explained here rather than silently applied.
		coolingNotice = updated.state === 'edited_since_recall';
		// The blind write is over, so the source may be looked at again.
		slot.unlock();
		mode = 'contrast';
	}

	/** Step 4. */
	async function resolveContrast(gaps: string[]) {
		if (!note) return;

		await Promise.all(
			gaps.map((text) => ipc.createQuestion(text, 'recall_gap', topicId, note!.id))
		);

		const updated = await ipc.recordContrast(note.id);
		note = updated;
		onsaved?.(updated);

		outgoing = null;
		mode = 'off';
	}

	/**
	 * Typing `@` opens the picker.
	 *
	 * A dialog rather than an inline autocomplete, and that is the point: the
	 * constraint is that mentions are always deliberate. There are no unlinked
	 * mention suggestions, no automatic linking, and no prompt telling you that
	 * two notes look related — linking everywhere buries the connections that
	 * actually matter.
	 *
	 * Disabled in memory mode: a picker listing note titles is a lookup, and a
	 * lookup during a blind write is exactly what memory mode forbids.
	 */
	function onEditorKeydown(event: KeyboardEvent) {
		if (event.key !== '@' || mode === 'writing') return;
		event.preventDefault();
		picking = 'mention';
		pickerOpen = true;
	}

	/**
	 * A mention is a link whose target is a note rather than a URL, so it
	 * survives copy, paste and export like any other link, and adds no new node
	 * type to migrate later.
	 */
	function insertMention(id: string | null) {
		if (!id || !editor) return;

		ipc.getNote(id).then((target) => {
			if (!target || !editor) return;
			const label = target.title.trim() || 'Untitled';

			editor
				.chain()
				.focus()
				.insertContent({
					type: 'text',
					text: label,
					marks: [{ type: 'link', attrs: { href: 'note:' + id } }]
				})
				.insertContent(' ')
				.run();

			schedule();
		});
	}

	function onPicked(id: string | null) {
		if (picking === 'mention') {
			insertMention(id);
			picking = 'source';
			return;
		}
		pickSource(id);
	}

	async function pickSource(id: string | null) {
		if (!note) return;
		const updated = await ipc.setNoteSource(note.id, id);
		note = updated;
		slot.enterNote(updated.id, updated.source_note_id, () => {
					picking = 'source';
					pickerOpen = true;
				});
		onsaved?.(updated);
	}

	function close() {
		clearTimeout(timer);
		save().finally(onclose);
	}

	/**
	 * The lock belongs to this editor's lifetime, not to its close button.
	 *
	 * Releasing it only in `close()` meant every other way of ending the editor
	 * — navigating off the board, or being remounted when you follow a backlink
	 * — left the panel locked shut with nothing on screen able to unlock it. The
	 * inspector then stayed hidden everywhere, for the rest of the session, with
	 * no way back. Tearing down is the one moment guaranteed to happen, so the
	 * release goes here.
	 */
	onDestroy(() => {
		clearTimeout(timer);
		slot.unlock();
		slot.leaveNote();
	});

	const statusLine = $derived(
		note
			? STATE_MEANING[note.state] +
					(note.never_contrasted ? ' Never contrasted against a source.' : '')
			: ''
	);

</script>

<EditorShell onclose={close}>
	{#snippet children({ presentationToggle }: { presentationToggle: import('svelte').Snippet })}

		<!-- State strip. States a fact; never nags. -->
		<div
			class="flex shrink-0 items-center gap-2 border-b px-6 py-2.5"
			style="border-color: var(--hairline)"
		>
			{#if note}
				<StateMark state={note.state} neverContrasted={note.never_contrasted} />
				<span class="text-muted-foreground text-[11px]">{statusLine}</span>
			{/if}

			<span class="ml-auto font-mono text-[9px] tracking-widest uppercase opacity-35">
				{mode === 'off' ? (dirty ? 'saving' : 'saved') : ''}
			</span>

			{#if mode !== 'contrast'}
				<button
					type="button"
					onclick={() => (confirming = mode === 'writing' ? 'exit' : 'enter')}
					class="flex items-center gap-1.5 rounded px-2 py-1 text-[11px] font-medium transition-colors
						{mode === 'writing' ? 'bg-foreground text-background' : 'hover:bg-accent'}"
				>
					{#if mode === 'writing'}
						<EyeOffIcon class="size-3.5" />
						Memory mode
					{:else}
						<EyeIcon class="size-3.5" />
						Memory mode off
					{/if}
				</button>
			{/if}

			<!-- Switch between filling the working area and a centred dialog, and
			     close. Both live here so the editor has exactly one chrome row. -->
			{@render presentationToggle()}
		</div>

		<!-- The consequence, shown before it happens rather than after. -->
		{#if confirming}
			<div
				class="bg-muted/40 shrink-0 border-b px-6 py-3"
				style="border-color: var(--hairline)"
			>
				<p class="mb-2 text-[12px] leading-relaxed">
					{#if confirming === 'enter'}
						The source disappears and the editor empties. Write what you remember — the
						previous version comes back afterwards, side by side, so you can compare.
					{:else}
						The source comes back on screen. What you have written stays, but this
						attempt can no longer reach <em>recalled</em> — you would be finishing it with
						the answer in front of you.
					{/if}
				</p>
				<div class="flex items-center gap-2">
					<Button
						size="sm"
						class="h-7 text-[11px]"
						onclick={confirming === 'enter' ? enterMemoryMode : leaveMemoryMode}
					>
						{confirming === 'enter' ? 'Write from memory' : 'Show the source'}
					</Button>
					<Button
						variant="ghost"
						size="sm"
						class="h-7 text-[11px]"
						onclick={() => (confirming = null)}
					>
						Cancel
					</Button>
				</div>
			</div>
		{/if}

		{#if coolingNotice}
			<div class="shrink-0 border-b px-6 py-2.5" style="border-color: var(--hairline)">
				<p class="text-[11px] leading-relaxed" style="color: var(--state-action)">
					Recorded — but you edited this within the last day, so it still reads as edited
					since recall. Reciting something you wrote hours ago tests the wrong thing. Try
					again tomorrow and it will count.
				</p>
			</div>
		{/if}

		{#if mode === 'contrast' && note}
			<ContrastStep {note} {outgoing} onresolve={resolveContrast} />
		{:else}
			<div class="flex min-h-0 grow flex-col">
				<div class="flex min-h-0 grow flex-col overflow-y-auto">
					<div class="px-8 pt-7">
						<input
							bind:value={title}
							oninput={schedule}
							placeholder="Untitled"
							class="w-full bg-transparent font-serif text-[28px] leading-tight font-semibold tracking-tight outline-none placeholder:opacity-20"
						/>
					</div>

					{#if editor}
						<Edra {editor}>
							<Edra.BubbleMenu />
							<Edra.Content class="edra-surface cursor-text px-8 pb-24 *:outline-none" />
							<Edra.DragHandle />
						</Edra>
					{/if}
				</div>
			</div>

			<footer
				class="flex shrink-0 items-center gap-2 border-t px-6 py-2.5"
				style="border-color: var(--hairline)"
			>
				{#if mode === 'writing'}
					<Button size="sm" class="h-7 text-[11px]" onclick={saveFromMemory}>
						Save from memory
					</Button>
				{:else}
					<!-- Present but disabled, so the path to `recalled` is always visible
					     and the reason it is closed is one hover away. -->
					<Tooltip.Provider delayDuration={200}>
						<Tooltip.Root>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<span {...props} class="inline-block">
										<Button size="sm" class="h-7 text-[11px]" disabled>
											Save from memory
										</Button>
									</span>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Content side="top" class="max-w-72 text-[11px] leading-relaxed">
								{#if disqualified}
									You turned memory mode off during this attempt, so it stops at draft.
									Reopen the note to try again.
								{:else}
									The source is on screen. Writing with it visible is reorganising, not
									remembering — turn on memory mode to reach recalled.
								{/if}
							</Tooltip.Content>
						</Tooltip.Root>
					</Tooltip.Provider>
				{/if}

				{#if note && note.kind === 'long' && mode === 'off'}
					<Button
						variant="ghost"
						size="sm"
						class="h-7 text-[11px]"
						onclick={async () => {
							if (!note) return;
							const updated = await ipc.setNoteKind(note.id, 'atomic');
							note = updated;
							onsaved?.(updated);
						}}
					>
						Mark atomic
					</Button>
				{/if}

				{#if editor && mode === 'off'}
					<!-- The fast lane, and it never bends a rule: with the source in
					     front of you this produces a draft, not a recall. -->
					<DistilHandle {editor} />
				{/if}

				<span class="text-muted-foreground ml-auto font-mono text-[10px]">
					<kbd>/</kbd> for blocks · select to format
				</span>
			</footer>
		{/if}
	{/snippet}
</EditorShell>

{#if note}
	<NotePicker bind:open={pickerOpen} excludeId={note.id} onpick={onPicked} />
{/if}
