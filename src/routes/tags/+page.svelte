<script lang="ts">
	/**
	 * Every tag, and what carries it.
	 *
	 * A count appears here because it is a *size*, not a backlog — how many
	 * notes carry a tag does not grow while you sleep, and it is the one number
	 * that makes a tag list usable at all: a tag on two notes is a tag you
	 * should probably drop.
	 *
	 * Picking one opens the library filtered by it. Tags classify; they do not
	 * contain, so there is no page belonging to a tag to land on.
	 */
	import * as ipc from '$lib/ipc';
	import { chrome } from '$lib/state/chrome.svelte';
	import BarTitle from '$lib/components/BarTitle.svelte';
	import { menu } from '$lib/state/menu.svelte';
	import { data } from '$lib/state/data.svelte';
	import HashIcon from '@lucide/svelte/icons/hash';

	let tags = $state<ipc.TagCount[]>([]);

	$effect(() => {
		data.tags;
		ipc.allTags().then((found) => (tags = found));
	});

	/**
	 * Dropping a tag takes it off every note at once.
	 *
	 * The counterpart to removing it from one note in the Info panel, which only
	 * lets the tag go once its last carrier does. A classification you have
	 * stopped using should not have to be hunted down note by note — and the
	 * notes themselves are untouched, which is what the wording has to say.
	 */
	function tagMenu(event: MouseEvent, tag: ipc.TagCount) {
		menu.show(event, [
			{
				label: 'Drop this tag',
				destructive: true,
				onpick: async () => {
					const ok = confirm(
						`Drop "${tag.name}"? It comes off ${tag.notes} note${tag.notes === 1 ? '' : 's'}. The notes stay.`
					);
					if (!ok) return;
					tags = tags.filter((x) => x.id !== tag.id);
					await ipc.deleteTag(tag.id);
				}
			}
		]);
	}

	$effect(() => chrome.claim(bar));
</script>

{#snippet bar()}<BarTitle>Tags</BarTitle>{/snippet}

<svelte:head><title>Tags — study</title></svelte:head>

<div class="h-full overflow-y-auto">
	<div class="mx-auto flex max-w-3xl flex-col px-10 py-12">
		<h1 class="text-[26px] font-semibold tracking-tight">Tags</h1>
		<p class="text-muted-foreground mt-1.5 text-[12.5px] leading-relaxed">
			A classification you apply on purpose. Picking one filters the library.
		</p>

		<div class="mt-8 flex flex-wrap gap-2">
			{#each tags as tag (tag.id)}
				<a
					href="/library?tag={tag.id}"
					oncontextmenu={(e) => tagMenu(e, tag)}
					class="bg-card hover:border-foreground/25 flex items-center gap-2 rounded-lg border px-3 py-2 transition-colors"
					style="border-color: var(--hairline)"
				>
					<HashIcon class="size-3 shrink-0 opacity-25" />
					<span class="text-[13px]">{tag.name}</span>
					<span class="font-mono text-[10px] tabular-nums opacity-30">{tag.notes}</span>
				</a>
			{:else}
				<p class="text-muted-foreground text-[13px] leading-relaxed">
					No tags yet. Add one from a note's Info panel.
				</p>
			{/each}
		</div>
	</div>
</div>
