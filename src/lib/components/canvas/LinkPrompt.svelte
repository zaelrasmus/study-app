<script lang="ts">
	/**
	 * Where you type the address of a page to put on the board.
	 *
	 * A small field rather than `prompt()`: the native dialog is not guaranteed
	 * in a Tauri webview, it cannot be styled, and it blocks the whole window
	 * for something that should cost one line of typing.
	 */
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';

	let { onsubmit, oncancel }: { onsubmit: (url: string) => void; oncancel: () => void } =
		$props();

	let url = $state('');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="absolute inset-0 z-40 flex items-center justify-center"
	style="background: color-mix(in oklch, var(--background) 45%, transparent)"
	transition:fade={{ duration: 120 }}
	onclick={oncancel}
	onkeydown={(e) => e.key === 'Escape' && oncancel()}
	role="presentation"
>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="bg-card w-[420px] max-w-[80%] rounded-lg border p-4 shadow-lg"
		style="border-color: var(--hairline-strong)"
		transition:scale={{ duration: 140, start: 0.97, easing: cubicOut }}
		onclick={(e) => e.stopPropagation()}
		role="presentation"
	>
		<p class="label mb-2">Add a link</p>

		<!-- svelte-ignore a11y_autofocus -->
		<input
			autofocus
			bind:value={url}
			placeholder="example.com/the-paper"
			class="bg-background focus:border-foreground h-9 w-full rounded-md border px-3 text-[13px] outline-none transition-colors"
			style="border-color: var(--hairline)"
			onkeydown={(e) => {
				if (e.key === 'Enter' && url.trim()) onsubmit(url.trim());
				if (e.key === 'Escape') oncancel();
			}}
		/>

		<p class="text-muted-foreground mt-2.5 text-[11.5px] leading-relaxed">
			Nothing is downloaded. The card points outwards, and opens in your browser.
		</p>

		<div class="mt-4 flex justify-end gap-2">
			<button
				type="button"
				onclick={oncancel}
				class="hover:bg-accent text-muted-foreground rounded-md px-3 py-1.5 text-[12px] transition-colors"
			>
				Cancel
			</button>
			<button
				type="button"
				disabled={!url.trim()}
				onclick={() => onsubmit(url.trim())}
				class="bg-foreground text-background rounded-md px-3.5 py-1.5 text-[12px] font-medium transition-opacity hover:opacity-85 disabled:opacity-25"
			>
				Add it
			</button>
		</div>
	</div>
</div>
