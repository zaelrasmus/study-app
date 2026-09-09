<script lang="ts">
	/**
	 * The name of the screen, in the top bar.
	 *
	 * Every destination claims the bar with one of these, because the fallback
	 * — the breadcrumb — belongs to boards and would otherwise sit there naming
	 * the last board you visited while you look at something else entirely.
	 */
	let { children }: { children: import('svelte').Snippet } = $props();
</script>

<!-- Fills the bar rather than hugging the text, so the empty stretch beside the
     name is drag region and not a dead strip. -->
<span data-tauri-drag-region class="ml-2.5 min-w-0 grow truncate text-[12.5px] font-medium">
	{@render children()}
</span>
