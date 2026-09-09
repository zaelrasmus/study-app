<script lang="ts">
	/**
	 * Every open task.
	 *
	 * Tasks are an inline block in any note, not a thing you file somewhere.
	 * This screen only makes them findable — and it shows **all** of them,
	 * including the ones with no `due:` date.
	 *
	 * That is a correction. Both queries used to ask for one specific day, so a
	 * checkbox typed into the journal without a date was indexed and then
	 * reachable from nowhere, which reads as the app quietly losing your
	 * writing. Losing a task is worse than a list being longer than you would
	 * like.
	 *
	 * The bounded view still exists, and it is the **panel**: that stays one day
	 * wide, which is where "a day's worth is work, a backlog is a reproach" does
	 * its job. A screen you went to deliberately is a different thing from a
	 * number sitting in the corner of your eye.
	 */
	import * as ipc from '$lib/ipc';
	import { slot } from '$lib/state/slot.svelte';
	import { chrome } from '$lib/state/chrome.svelte';
	import BarTitle from '$lib/components/BarTitle.svelte';
	import { today as todayIso } from '$lib/date';
	import NoteEditor from '$lib/components/NoteEditor.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { data } from '$lib/state/data.svelte';

	const today = todayIso();

	let tasks = $state<ipc.DueTask[]>([]);
	let editingNoteId = $state<string | null>(null);

	async function load() {
		tasks = await ipc.allTasks();
	}

	$effect(() => {
		data.tasks;
		data.notes;
		load();
	});

	/**
	 * Three bands, and no fourth for "overdue".
	 *
	 * Something dated last Tuesday and still open is work for now, not a
	 * separate category of failure — so it sits in Now with today's, rather than
	 * under a heading whose only content is that you are late.
	 */
	const bands = $derived([
		{
			id: 'now',
			label: 'Now',
			hint: 'Due today, or dated earlier and still open',
			items: tasks.filter((t) => t.due_on !== null && t.due_on <= today)
		},
		{
			id: 'later',
			label: 'Later',
			hint: 'Dated ahead',
			items: tasks.filter((t) => t.due_on !== null && t.due_on > today)
		},
		{
			id: 'anytime',
			label: 'Anytime',
			hint: 'Written without a date',
			items: tasks.filter((t) => t.due_on === null)
		}
	].filter((band) => band.items.length > 0));

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

	function when(task: ipc.DueTask) {
		if (!task.due_on) return '';
		if (task.due_on === today) return 'today';
		return new Date(`${task.due_on}T00:00:00`).toLocaleDateString(undefined, {
			day: 'numeric',
			month: 'short'
		});
	}

	const clean = (text: string) => text.replace(/due:\d{4}-\d{2}-\d{2}/, '').trim();

	/**
	 * Ticking one edits the note's document, so the body stays the only truth —
	 * it is the same edit you would make by clicking the box in the editor.
	 */
	async function complete(task: ipc.DueTask) {
		tasks = tasks.filter((t) => t.id !== task.id);
		await ipc.setTaskDone(task.id, true);
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
		pending = setTimeout(() => complete(task), 220);
	}

	function onDouble(task: ipc.DueTask) {
		clearTimeout(pending);
		editingNoteId = task.note_id;
	}

	function taskMenu(event: MouseEvent, task: ipc.DueTask) {
		menu.show(event, [
			{ label: 'Open the note it lives in', onpick: () => (editingNoteId = task.note_id) },
			{ label: 'Mark it done', onpick: () => complete(task) },
			{
				label: 'Delete the note',
				destructive: true,
				onpick: async () => {
					if (!confirm(`Delete "${where(task)}"? The whole note goes. This cannot be undone.`))
						return;
					tasks = tasks.filter((t) => t.note_id !== task.note_id);
					await ipc.deleteNote(task.note_id);
				}
			}
		]);
	}

	$effect(() => {
		slot.openNote = (id: string) => (editingNoteId = id);
		return () => (slot.openNote = undefined);
	});

	$effect(() => chrome.claim(bar));
</script>

{#snippet bar()}<BarTitle>Tasks</BarTitle>{/snippet}

<svelte:head><title>Tasks — study</title></svelte:head>

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-2xl flex-col px-10 py-12">
		<h1 class="text-[26px] font-semibold tracking-tight">Tasks</h1>
		<p class="text-muted-foreground mt-1.5 text-[12.5px] leading-relaxed">
			Written inside notes, gathered here. Add <code class="font-mono text-[12px]">due:{today}</code>
			to one and it also turns up in the panel for that day.
		</p>

		<div class="mt-9 flex flex-col gap-8">
			{#each bands as band (band.id)}
				<section class="flex flex-col">
					<header class="mb-2 flex items-baseline gap-3">
						<h2 class="label">{band.label}</h2>
						<span class="text-muted-foreground truncate text-[11.5px]">{band.hint}</span>
					</header>

					<div class="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-2.5">
						{#each band.items as task (task.id)}
							<button
								type="button"
								onclick={() => onSingle(task)}
								ondblclick={() => onDouble(task)}
								oncontextmenu={(e) => taskMenu(e, task)}
								title="Click to complete · double-click to open the note"
								class="group bg-card hover:border-foreground/25 flex flex-col gap-2 rounded-lg border p-3.5 text-left transition-colors"
								style="border-color: var(--hairline)"
							>
								<div class="flex items-start gap-2.5">
									<!-- A real box: ticking it writes back into the note's
									     document, which is the same edit you would make by
									     clicking it in the editor. -->
									<span
										class="group-hover:border-foreground/50 mt-[3px] size-3 shrink-0 rounded-[3px] border transition-colors"
										style="border-color: var(--hairline-strong)"
										aria-hidden="true"
									></span>
									<span class="min-w-0 grow text-[13px] leading-snug">
										{clean(task.text)}
									</span>
								</div>

								<div class="text-muted-foreground flex items-center gap-2 pl-[22px] font-mono text-[9.5px]">
									<span class="min-w-0 truncate">{where(task)}</span>
									{#if when(task)}
										<span class="opacity-30">·</span>
										<span class="shrink-0">{when(task)}</span>
									{/if}
								</div>
							</button>
						{/each}
					</div>
				</section>
			{:else}
				<p class="text-muted-foreground text-[13px] leading-relaxed">
					Nothing open. Type a checkbox in any note and it turns up here.
				</p>
			{/each}
		</div>
	</div>
</div>

{#if editingNoteId}
	{#key editingNoteId}
		<NoteEditor
			noteId={editingNoteId}
			onclose={() => (editingNoteId = null)}
			onsaved={() => load()}
		/>
	{/key}
{/if}
