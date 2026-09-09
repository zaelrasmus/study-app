<script lang="ts">
	/**
	 * Another topic, sitting on this one's canvas.
	 *
	 * This is zoom, not a folder tree: there is no path to this topic and no
	 * parent recorded anywhere. It is here because you put it here, and the same
	 * topic can sit on several canvases at once.
	 *
	 * Double-click opens it, matching notes — a single click would fire while you
	 * were only trying to select the thing.
	 */
	import { type NodeProps } from '@xyflow/svelte';
	import CornerDownRightIcon from '@lucide/svelte/icons/corner-down-right';

	type Data = { title: string; onopen: () => void; oncontext: (event: MouseEvent) => void };

	let { data, selected }: NodeProps = $props();

	const title = $derived((data as Data).title);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	class="bg-card flex h-full w-full cursor-default items-center gap-2 rounded-md px-3 py-2.5 transition-all"
	style="border: 1px solid {selected ? 'var(--foreground)' : 'var(--hairline-strong)'};
		box-shadow: 2px 2px 0 0 {selected ? 'var(--hairline-strong)' : 'var(--hairline)'}"
	ondblclick={() => (data as Data).onopen()}
	oncontextmenu={(e) => (data as Data).oncontext(e)}
>
	<CornerDownRightIcon class="size-3 shrink-0 opacity-35" />
	<h3 class="min-w-0 grow truncate text-[12px] font-medium tracking-tight">{title}</h3>
	<span class="label !text-[8px]">topic</span>
</article>
