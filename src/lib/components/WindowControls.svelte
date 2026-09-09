<script lang="ts">
	/**
	 * Minimise, maximise, close.
	 *
	 * Fixed to the top-right of the window rather than living inside a titlebar
	 * row, because the sidebars now run the full height and there is no full-width
	 * strip left to put them in. Fixed positioning also means they can never be
	 * pushed out of view by a layout that overflows — which has happened once
	 * already, and with OS decorations off these are the only close button there
	 * is.
	 */
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { onMount } from 'svelte';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import SquareIcon from '@lucide/svelte/icons/square';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import XIcon from '@lucide/svelte/icons/x';

	/** Guarded: an exception here would leave the window with no way out. */
	function win() {
		try {
			return getCurrentWindow();
		} catch {
			return null;
		}
	}

	let maximized = $state(false);

	onMount(() => {
		const w = win();
		if (!w) return;

		w.isMaximized().then((v) => (maximized = v));
		const stop = w.onResized(() => {
			w.isMaximized().then((v) => (maximized = v));
		});

		return () => {
			stop.then((fn) => fn());
		};
	});
</script>

<div class="bg-sidebar fixed top-0 right-0 z-[80] flex">
	<button
		type="button"
		onclick={() => win()?.minimize()}
		title="Minimise"
		class="hover:bg-accent flex h-8 w-11 items-center justify-center opacity-45 transition-all hover:opacity-100"
	>
		<MinusIcon class="size-3.5" />
	</button>
	<button
		type="button"
		onclick={() => win()?.toggleMaximize()}
		title={maximized ? 'Restore' : 'Maximise'}
		class="hover:bg-accent flex h-8 w-11 items-center justify-center opacity-45 transition-all hover:opacity-100"
	>
		{#if maximized}
			<CopyIcon class="size-3" />
		{:else}
			<SquareIcon class="size-3" />
		{/if}
	</button>
	<!-- The only hard-coded colour in the app, and a platform convention. -->
	<button
		type="button"
		onclick={() => win()?.close()}
		title="Close"
		class="flex h-8 w-11 items-center justify-center opacity-45 transition-all hover:bg-[#e81123] hover:text-white hover:opacity-100"
	>
		<XIcon class="size-3.5" />
	</button>
</div>
