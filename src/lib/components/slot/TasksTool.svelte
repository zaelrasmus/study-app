<script lang="ts">
	/**
	 * What is due on the day the journal is showing.
	 *
	 * It takes its date from the centre rather than always meaning "today", so
	 * arrowing back through the week moves the panel with the page. A panel that
	 * silently described a different day from the one on screen would be worse
	 * than no panel.
	 *
	 * Two slices, both a single day wide: tasks **dated** for that day wherever
	 * they were written, and tasks written **on that day's own page**, dated or
	 * not. The second exists because typing three checkboxes into today's journal
	 * and finding this panel empty beside them reads as the app not seeing what
	 * is plainly on screen.
	 *
	 * It stays one day either way, which is the whole constraint: "everything
	 * still open" is a number that grows while you sleep, and that is the
	 * accumulating debt the review queue already refuses. An undated task does
	 * not follow you into tomorrow — the Tasks screen is where all of them live.
	 */
	import * as ipc from '$lib/ipc';
	import { today as todayIso } from '$lib/date';
	import { menu } from '$lib/state/menu.svelte';
	import { data } from '$lib/state/data.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import PanelCard from './PanelCard.svelte';

	let { day = null }: { day?: string | null } = $props();

	const date = $derived(day ?? todayIso());
	let tasks = $state<ipc.DueTask[]>([]);

	$effect(() => {
		// Typing a checkbox in the note beside this panel is a write to a
		// note, so both kinds matter.
		data.tasks;
		data.notes;
		ipc.tasksDue(date).then((found) => (tasks = found));
	});

	/**
	 * Ticking one, from here — and unticking it.
	 *
	 * This edits the note's document, so the body stays the single truth. What
	 * it no longer does is take the row away. A task that vanishes when you
	 * finish it removes the only evidence the list is moving, at the exact
	 * moment that evidence is worth having: three ticked rows sitting there is
	 * what makes the fourth one feel worth starting. The panel is one day wide,
	 * so nothing accumulates.
	 *
	 * It also stays exactly where it was. Sorting the finished ones to the
	 * bottom would be a gentler kind of disappearing — the row you just touched
	 * would still slide out from under the pointer.
	 *
	 * Flipped locally first so the click feels immediate; the write follows.
	 */
	async function toggle(task: ipc.DueTask) {
		const next = !task.done;
		task.done = next;

		// If an editor has this note open, its copy of the document is the live
		// one, and writing to the database behind it would be undone the moment
		// it next saved. Ticking the box in the editor is the same edit made in
		// the place that owns it — and you watch it happen in the note beside
		// this panel.
		if (slot.setTaskInOpenDocument(task.id, next)) return;

		await ipc.setTaskDone(task.id, next);
	}

	/**
	 * A single click is a different act from the first half of a double click.
	 *
	 * The browser fires `click` before `dblclick`, so without this, opening the
	 * note would tick the task on the way. Holding the single-click action for a
	 * moment lets the second click cancel it.
	 */
	let pending: ReturnType<typeof setTimeout> | undefined;

	function onSingle(task: ipc.DueTask) {
		clearTimeout(pending);
		pending = setTimeout(() => toggle(task), 220);
	}

	function onDouble(task: ipc.DueTask) {
		clearTimeout(pending);
		slot.openNote?.(task.note_id);
	}

	function taskMenu(event: MouseEvent, task: ipc.DueTask) {
		menu.show(event, [
			{ label: 'Open the note it lives in', onpick: () => slot.openNote?.(task.note_id) },
			{ label: task.done ? 'Put it back' : 'Mark it done', onpick: () => toggle(task) },
			{
				label: 'Delete the note',
				destructive: true,
				onpick: async () => {
					const face = where(task);
					if (!confirm(`Delete "${face}"? The whole note goes. This cannot be undone.`))
						return;
					tasks = tasks.filter((t) => t.note_id !== task.note_id);
					await ipc.deleteNote(task.note_id);
				}
			}
		]);
	}

	/** A day's page has no title, so the date is what it is called. */
	function where(task: ipc.DueTask) {
		if (task.journal_day) {
			return new Date(`${task.journal_day}T00:00:00`).toLocaleDateString(undefined, {
				day: 'numeric',
				month: 'long'
			});
		}
		return task.note_title || 'Untitled';
	}

	/** Grouped by note, so a list of tasks still says where each one lives. */
	const byNote = $derived(
		Object.entries(
			tasks.reduce<Record<string, ipc.DueTask[]>>((acc, task) => {
				(acc[where(task)] ??= []).push(task);
				return acc;
			}, {})
		)
	);
</script>

<div class="min-h-0 overflow-y-auto px-3 py-2.5">
	{#each byNote as [noteTitle, items] (noteTitle)}
		<p class="label mt-3 mb-1.5 first:mt-0">{noteTitle}</p>
		{#each items as task (task.id)}
			<PanelCard
				onclick={() => onSingle(task)}
				ondblclick={() => onDouble(task)}
				oncontextmenu={(e) => taskMenu(e, task)}
				title={task.done
					? 'Click to put it back · double-click to open the note'
					: 'Click to complete · double-click to open the note'}
			>
				<div
					class="flex items-start gap-2 transition-opacity"
					style="opacity: {task.done ? 0.45 : 1}"
				>
					<!-- A real box now. It writes back into the note's document, so
					     the body is still the only truth — this is the same edit
					     you would make by clicking it in the editor.

					     Filled when done rather than gone: the mark is the point. -->
					<span
						class="mt-[4px] size-2.5 shrink-0 rounded-[2px] border transition-colors"
						style="border-color: var(--hairline-strong); background: {task.done
							? 'var(--foreground)'
							: 'transparent'}"
						aria-hidden="true"
					></span>
					<span
						class="min-w-0 grow text-[12px] leading-snug"
						style={task.done ? 'text-decoration: line-through' : ''}
					>
						{task.text.replace(/due:\d{4}-\d{2}-\d{2}/, '').trim()}
					</span>
					{#if task.due_on}
						<!-- Dated for this day, rather than merely written on it. -->
						<span class="label shrink-0 !text-[8px]" title="Dated for this day">due</span>
					{/if}
				</div>
			</PanelCard>
		{/each}
	{:else}
		<p class="text-muted-foreground px-1 py-3 text-[12px] leading-relaxed">
			Nothing on for {date === todayIso() ? 'today' : 'that day'}. This panel is one
			day wide: what you wrote on the day's page, and anything dated for it. Type
			<code class="font-mono text-[11px]">/</code> in a task to give it a date.
		</p>
	{/each}
</div>
