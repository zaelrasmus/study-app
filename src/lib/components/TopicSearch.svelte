<script lang="ts">
	/**
	 * How a sleeping topic comes back.
	 *
	 * There is no tree to browse anywhere in the app, so this is the browse
	 * surface -- and it is transient by design: it opens, you pick, it closes,
	 * and nothing accumulates on screen.
	 */
	import * as Command from '$lib/components/ui/command';
	import * as ipc from '$lib/ipc';
	import type { Topic } from '$lib/types';
	import { goto } from '$app/navigation';

	let { open = $bindable(false) }: { open?: boolean } = $props();

	let topics = $state<Topic[]>([]);

	$effect(() => {
		if (open) {
			ipc.allTopics().then((t) => (topics = t));
		}
	});

	function pick(topic: Topic) {
		open = false;
		goto(`/t/${topic.id}`);
	}
</script>

<Command.Dialog bind:open>
	<Command.Input placeholder="Find a topic" />
	<Command.List>
		<Command.Empty>Nothing by that name.</Command.Empty>
		<Command.Group heading="Topics">
			{#each topics as topic (topic.id)}
				<Command.Item value={topic.title} onSelect={() => pick(topic)}>
					{topic.title}
				</Command.Item>
			{/each}
		</Command.Group>
	</Command.List>
</Command.Dialog>
