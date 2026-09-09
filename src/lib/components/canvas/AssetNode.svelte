<script lang="ts">
	/**
	 * A file on the board: an image, or a PDF.
	 *
	 * Not a note. It carries no title, no knowledge state and no card, and it
	 * never enters review — it is what you think *about*, not what you have
	 * learned. Distilling one into a note is a separate, deliberate act, the
	 * same way loose text becomes a note only when you say so.
	 *
	 * An image is shown at whatever size you resize it to, with no chrome at
	 * all: on a board full of your own writing, a diagram should look like the
	 * diagram and not like a widget wrapping one. A PDF cannot be shown that
	 * way — it needs its own scroll and its own page furniture — so it gets a
	 * frame and a name, and the frame is honest that there is an application
	 * inside it.
	 */
	import { Handle, NodeResizer, Position, type NodeProps } from '@xyflow/svelte';
	import type { Asset } from '$lib/types';
	import FileIcon from '@lucide/svelte/icons/file-text';

	type Data = {
		asset: Asset;
		/** Already resolved to something the webview will load. */
		src: string;
		connecting: boolean;
		oncontext: (event: MouseEvent) => void;
	};

	let { data, selected }: NodeProps = $props();

	const asset = $derived((data as Data).asset);
	const src = $derived((data as Data).src);
	const connecting = $derived((data as Data).connecting);
	const isPdf = $derived(asset.kind === 'pdf');
</script>

<NodeResizer
	isVisible={selected}
	minWidth={120}
	minHeight={90}
	keepAspectRatio={!isPdf}
	lineClass="!border-transparent"
	handleClass="!size-1.5 !rounded-none !border-foreground/40 !bg-background"
/>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<figure
	class="relative h-full w-full overflow-hidden rounded-md transition-all"
	style="border: 1px solid {selected ? 'var(--foreground)' : isPdf ? 'var(--hairline-strong)' : 'transparent'};
		background: {isPdf ? 'var(--card)' : 'transparent'}"
	oncontextmenu={(e) => (data as Data).oncontext(e)}
>
	{#if isPdf}
		<figcaption
			class="flex h-7 shrink-0 items-center gap-1.5 border-b px-2.5"
			style="border-color: var(--hairline)"
		>
			<FileIcon class="size-3 shrink-0 opacity-40" />
			<span class="min-w-0 grow truncate text-[11px] font-medium">{asset.file_name}</span>
		</figcaption>

		<!-- The webview's own PDF viewer. It swallows pointer events, which is
		     what you want when reading and not when arranging — so it is inert
		     until the card is selected. -->
		<iframe
			title={asset.file_name}
			{src}
			class="h-[calc(100%-1.75rem)] w-full border-0"
			style="pointer-events: {selected ? 'auto' : 'none'}"
		></iframe>
	{:else}
		<img
			{src}
			alt={asset.file_name}
			draggable="false"
			class="pointer-events-none h-full w-full object-contain"
		/>
	{/if}
</figure>

<!-- Only reachable while the connect tool is on, so ordinary dragging is never
     hijacked by a handle sitting under the pointer. -->
<Handle type="target" position={Position.Left} class={connecting ? 'canvas-port' : ''} />
<Handle type="source" position={Position.Right} class={connecting ? 'canvas-port' : ''} />
