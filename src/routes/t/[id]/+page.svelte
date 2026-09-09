<script lang="ts">
	/**
	 * The canvas route.
	 *
	 * Its only job is to provide the flow store. `useSvelteFlow()` reads that
	 * store from Svelte context and throws when no `SvelteFlowProvider` is an
	 * ancestor, so every component that needs flow coordinates — the canvas
	 * itself, the ink layer, the frame marquee — has to sit inside this wrapper.
	 */
	import { SvelteFlowProvider } from '@xyflow/svelte';
	import { page } from '$app/state';
	import TopicCanvas from '$lib/components/canvas/TopicCanvas.svelte';
	import { session } from '$lib/state/session.svelte';
	import type { Topic } from '$lib/types';

	const topicId = $derived(page.params.id!);

	let topic = $state<Topic | null>(null);

	$effect(() => {
		session.crumbs = topic ? [{ label: topic.title || 'Untitled topic' }] : [];
	});
</script>

<svelte:head><title>{topic?.title ?? 'Canvas'} — study</title></svelte:head>

<!-- `key` forces a fresh provider and a fresh canvas per topic, so nothing from
     the previous topic's flow state can bleed across. -->
{#key topicId}
	<SvelteFlowProvider>
		<TopicCanvas {topicId} ontopic={(t) => (topic = t)} />
	</SvelteFlowProvider>
{/key}
