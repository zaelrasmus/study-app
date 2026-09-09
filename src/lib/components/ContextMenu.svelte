<script lang="ts">
	/**
	 * The one context menu.
	 *
	 * Destructive items sit below a rule and carry the action colour, so
	 * "remove from this board" and "delete everywhere" can never be misread as
	 * the same weight of decision.
	 */
	import { menu } from '$lib/state/menu.svelte';
	import { fill } from '$lib/cardColour';
</script>

{#if menu.open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="fixed inset-0 z-[90]"
		onpointerdown={() => menu.close()}
		oncontextmenu={(e) => {
			e.preventDefault();
			menu.close();
		}}
	></div>

	<div
		class="bg-popover fixed z-[91] min-w-52 rounded-md border py-1 shadow-lg"
		style="left: {menu.x}px; top: {menu.y}px; border-color: var(--hairline-strong)"
	>
		{#each menu.items as item, i (i)}
			{#if item.swatches}
				<div class="flex items-center gap-1 px-2.5 py-1.5">
					{#each item.swatches as swatch (swatch.label)}
						<button
							type="button"
							title={swatch.label}
							onclick={() => {
								item.onswatch?.(swatch.id);
								menu.close();
							}}
							class="size-5 rounded border transition-transform hover:scale-110"
							style="background: {fill(swatch.id) ?? 'transparent'};
								border-color: var(--card-edge)"
						>
							{#if !swatch.id}
								<span class="block h-full w-full text-[9px] opacity-40">·</span>
							{/if}
						</button>
					{/each}
				</div>
			{:else}
				{#if item.destructive && i > 0}
					<div class="my-1 border-t" style="border-color: var(--hairline)"></div>
				{/if}
				<button
					type="button"
					onclick={() => {
						item.onpick?.();
						menu.close();
					}}
					class="hover:bg-accent flex w-full items-center px-2.5 py-1.5 text-left text-[11.5px] transition-colors"
					style={item.destructive ? 'color: var(--state-action)' : undefined}
				>
					{item.label}
				</button>
			{/if}
		{/each}
	</div>
{/if}
