<script lang="ts">
	/**
	 * The strip above the working area.
	 *
	 * It spans only the middle column — the sidebars run past it to the top of
	 * the window — so it belongs to the centre, and the centre is a route. The
	 * route fills it: a week for the journal, filters for the library, a
	 * breadcrumb for a board.
	 *
	 * It also carries the button that brings the inspector back, because a closed
	 * inspector is gone rather than collapsed to a rail — so the control to
	 * reopen it cannot live inside the thing it reopens.
	 *
	 * The breadcrumb is the fallback, and it shows the path you *navigated*, not
	 * a canonical location. Topics sit on several boards at once, so "where am
	 * I" has no single answer, and pretending otherwise would be the folder tree
	 * returning by implication.
	 */
	import { session } from '$lib/state/session.svelte';
	import { chrome } from '$lib/state/chrome.svelte';
	import { slot } from '$lib/state/slot.svelte';
	import { goto } from '$app/navigation';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import PanelIcon from '@lucide/svelte/icons/panel-right';
</script>

<header
	data-tauri-drag-region
	class="flex h-10 shrink-0 items-center gap-0.5 border-b pl-1.5 select-none"
	style="border-color: var(--hairline)"
>
	<button
		type="button"
		onclick={() => history.back()}
		title="Back"
		class="hover:bg-accent flex size-7 shrink-0 items-center justify-center rounded-md opacity-40 transition-all hover:opacity-100"
	>
		<ChevronLeft class="size-4" />
	</button>
	<button
		type="button"
		onclick={() => history.forward()}
		title="Forward"
		class="hover:bg-accent flex size-7 shrink-0 items-center justify-center rounded-md opacity-40 transition-all hover:opacity-100"
	>
		<ChevronRight class="size-4" />
	</button>

	{#if chrome.bar}
		<!-- Drag region on the wrapper as well as the header.
		
		     Tauri tests the element *under the pointer* for the attribute — it is
		     not inherited — so a route filling this bar with its own content made
		     that whole stretch undraggable, and the window could only be moved by
		     the few pixels of header that showed through. Interactive children
		     (buttons, inputs) are the event target themselves, so they keep
		     working: only the gaps between them drag. -->
		<div data-tauri-drag-region class="flex min-w-0 grow items-center">
			{@render chrome.bar()}
		</div>
	{:else}
		<nav
			data-tauri-drag-region
			class="ml-2 flex min-w-0 grow items-center gap-1.5 text-[12.5px]"
		>
			{#each session.crumbs as crumb, i (crumb.label + i)}
				{#if i > 0}
					<span class="opacity-25">/</span>
				{/if}
				{#if crumb.href}
					<button
						type="button"
						onclick={() => goto(crumb.href!)}
						class="truncate opacity-50 transition-opacity hover:opacity-100"
					>
						{crumb.label}
					</button>
				{:else}
					<span class="truncate font-medium">{crumb.label}</span>
				{/if}
			{/each}
		</nav>
	{/if}

	<!-- Absent while the panel is open: the panel carries its own close button,
	     and two controls for one thing is one too many. -->
	{#if !slot.open}
		<button
			type="button"
			onclick={() => slot.toggle()}
			title="Show panel"
			class="hover:bg-accent mr-1 flex size-7 shrink-0 items-center justify-center rounded-md opacity-40 transition-all hover:opacity-100"
		>
			<PanelIcon class="size-4" />
		</button>
	{/if}

	<!-- Kept clear of the window controls. -->
	<div data-tauri-drag-region class="h-full w-[136px] shrink-0"></div>
</header>
