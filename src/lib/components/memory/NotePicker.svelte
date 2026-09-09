<script lang="ts">
	/**
	 * Picks the note being distilled from.
	 *
	 * Search runs in Rust so it folds Spanish properly — `localizacion` finds
	 * `localización`, and `ano` does not find `año`.
	 */
	import * as Command from '$lib/components/ui/command';
	import * as ipc from '$lib/ipc';
	import type { Note } from '$lib/types';

	let {
		open = $bindable(false),
		excludeId,
		onpick
	}: { open?: boolean; excludeId: string; onpick: (id: string | null) => void } = $props();

	let query = $state('');
	let results = $state<Note[]>([]);

	$effect(() => {
		if (!open) return;
		const q = query;
		// An empty query lists everything, so the picker is browsable too.
		ipc.searchNotes(q).then((found) => {
			results = found.filter((n) => n.id !== excludeId).slice(0, 40);
		});
	});
</script>

<Command.Dialog bind:open shouldFilter={false}>
	<Command.Input bind:value={query} placeholder="Find the note you are distilling from" />
	<Command.List>
		<Command.Empty>Nothing by that name.</Command.Empty>
		<Command.Group heading="Notes">
			{#each results as note (note.id)}
				<Command.Item
					value={note.id}
					onSelect={() => {
						onpick(note.id);
						open = false;
					}}
				>
					<span class="truncate">{note.title || 'Untitled'}</span>
				</Command.Item>
			{/each}
		</Command.Group>
		<Command.Group heading="None">
			<Command.Item
				value="__clear"
				onSelect={() => {
					onpick(null);
					open = false;
				}}
			>
				<span class="opacity-60">No source</span>
			</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Dialog>
