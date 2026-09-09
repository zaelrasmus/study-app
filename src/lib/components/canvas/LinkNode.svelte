<script lang="ts">
	/**
	 * A page on the web, sitting beside your cards.
	 *
	 * Deliberately **not** an asset: nothing was copied, and the page can change
	 * under you or vanish. So the card is honest about being a pointer outwards
	 * rather than something you now have — it shows the host, not a preview, and
	 * it makes no promise that what is there today will be there next year.
	 *
	 * It opens in your browser rather than in a webview here. A board is for
	 * arranging your own thinking; a page you are reading belongs somewhere you
	 * can use the whole screen for it.
	 */
	import { Handle, Position, type NodeProps } from '@xyflow/svelte';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import LinkIcon from '@lucide/svelte/icons/link';
	import ExternalIcon from '@lucide/svelte/icons/arrow-up-right';

	type Data = {
		url: string;
		connecting: boolean;
		oncontext: (event: MouseEvent) => void;
	};

	let { data, selected }: NodeProps = $props();

	const url = $derived((data as Data).url);
	const connecting = $derived((data as Data).connecting);

	/** The host is the honest summary. A full URL is a path, not a name. */
	const host = $derived.by(() => {
		try {
			return new URL(url).hostname.replace(/^www\./, '');
		} catch {
			return url;
		}
	});

	/** What comes after the host, when there is something worth showing. */
	const rest = $derived.by(() => {
		try {
			const u = new URL(url);
			const tail = decodeURIComponent(u.pathname + u.search).replace(/\/$/, '');
			return tail === '' || tail === '/' ? '' : tail;
		} catch {
			return '';
		}
	});

	function open() {
		openUrl(url).catch(() => {
			/* No browser to open it with is not something to shout about. */
		});
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	class="group bg-card flex h-full w-full cursor-default flex-col justify-center gap-1 rounded-md px-3 py-2.5 transition-all"
	style="border: 1px solid {selected ? 'var(--foreground)' : 'var(--hairline-strong)'}"
	title={url}
	ondblclick={open}
	oncontextmenu={(e) => (data as Data).oncontext(e)}
>
	<div class="flex items-center gap-2">
		<LinkIcon class="size-3 shrink-0 opacity-35" />
		<span class="min-w-0 grow truncate text-[12px] font-medium tracking-tight">{host}</span>
		<ExternalIcon
			class="size-3 shrink-0 opacity-0 transition-opacity group-hover:opacity-40"
		/>
	</div>

	{#if rest}
		<p class="text-muted-foreground truncate pl-5 text-[10.5px]">{rest}</p>
	{/if}
</article>

<!-- Only reachable while the connect tool is on, so ordinary dragging is never
     hijacked by a handle sitting under the pointer. -->
<Handle type="target" position={Position.Left} class={connecting ? 'canvas-port' : ''} />
<Handle type="source" position={Position.Right} class={connecting ? 'canvas-port' : ''} />
