<script lang="ts">
	/**
	 * The shell.
	 *
	 * Both sidebars run the full height of the window, flush with the top, and
	 * the top bar spans only the working area between them. That is the layout
	 * logic worth taking from Heptabase: the panels are the frame and the thing
	 * you are working on sits inside it, rather than a full-width chrome bar
	 * cutting across everything.
	 *
	 * The window controls are a fixed overlay at the top-right, because there is
	 * no full-width strip left to hold them — and with OS decorations off they
	 * are the only close button there is, so they must never depend on a layout
	 * that could overflow.
	 */
	import '../app.css';
	import { onMount } from 'svelte';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { ModeWatcher, mode } from 'mode-watcher';
	import { page } from '$app/state';
	import { Toaster } from '$lib/components/ui/sonner';

	import WindowControls from '$lib/components/WindowControls.svelte';
	import TopBar from '$lib/components/TopBar.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import Resizer from '$lib/components/Resizer.svelte';
	import RightSlot from '$lib/components/slot/RightSlot.svelte';
	import DragGhost from '$lib/components/DragGhost.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import CaptureDialog from '$lib/components/CaptureDialog.svelte';
	import TopicSearch from '$lib/components/TopicSearch.svelte';

	import { session } from '$lib/state/session.svelte';
	import { data } from '$lib/state/data.svelte';
	import { slot, type SlotContext } from '$lib/state/slot.svelte';
	import { prefs } from '$lib/state/prefs.svelte';

	let { children } = $props();

	// An open editor wins over the route: it is an overlay, so it can sit on the
	// canvas or on any screen, and either way Info and Source describe it.
	const slotContext = $derived<SlotContext>(
		slot.noteId
			? 'note'
			: page.url.pathname.startsWith('/t/')
				? 'canvas'
				: page.url.pathname.startsWith('/journal')
					? 'journal'
					: 'none'
	);



	let captureOpen = $state(false);
	let searchOpen = $state(false);

	onMount(() => {
		session.refresh();
	});

	/**
	 * The counts in the sidebar, and the unplaced list, come from one snapshot.
	 *
	 * Re-taken whenever anything they summarise changes, so the number beside
	 * Questions is not left describing a queue you already emptied.
	 */
	$effect(() => {
		data.notes;
		data.questions;
		data.review;
		data.boards;
		session.refresh();
	});

	/**
	 * The window's own ground, kept in step with the theme.
	 *
	 * With decorations off the webview paints nearly everything, but Tauri still
	 * owns the colour behind it — visible for a frame while dragging the window
	 * larger. Configured dark, so without this a light theme tears black at the
	 * edge as it grows.
	 */
	$effect(() => {
		const dark = mode.current === 'dark';
		getCurrentWindow()
			.setBackgroundColor(dark ? '#0a0a0a' : '#ffffff')
			.catch(() => {
				/* cosmetic; never worth an error */
			});
	});

	function onkeydown(event: KeyboardEvent) {
		if (!(event.ctrlKey || event.metaKey)) return;

		if (event.key === 'n') {
			event.preventDefault();
			captureOpen = true;
		}

		if (event.key === 'k') {
			event.preventDefault();
			searchOpen = true;
		}

		// The escape hatch: it does not depend on our own layout rendering.
		if (event.key === 'w') {
			event.preventDefault();
			getCurrentWindow()
				.close()
				.catch(() => {});
		}
	}
</script>

<svelte:window {onkeydown} />

<ModeWatcher />
<Toaster position="bottom-right" />

<div class="flex h-full w-full overflow-hidden">
	<div class="shrink-0 overflow-hidden" style="width: {prefs.sidebarWidth}px">
		<Sidebar onsearch={() => (searchOpen = true)} oncapture={() => (captureOpen = true)} />
	</div>

	<Resizer side="left" onresize={(px) => prefs.setSidebarWidth(px)} />

	<div class="flex min-w-0 grow flex-col overflow-hidden">
		<TopBar />
		<main class="relative min-h-0 grow overflow-hidden">
			{@render children()}
		</main>
	</div>

	<!-- Only a real, open slot is resizable; a rail and a lock have no width to set. -->
	{#if slot.open}
		<Resizer side="right" onresize={(px) => prefs.setSlotWidth(px)} />
	{/if}

	<!-- The board is passed only while one is actually in view: an "on this
	     board" filter for a board you have left is a lie. -->
	<RightSlot
		context={slotContext}
		topic={slotContext === 'canvas' ? slot.topic : null}
		frames={slotContext === 'canvas' ? slot.frames : []}
		sourceNoteId={slot.sourceNoteId}
		onfocusframe={slot.focusFrame}
	/>
</div>

<WindowControls />
<DragGhost />
<ContextMenu />

<!-- The board in view, so a doubt caught while working on it is *about* it.
     Capturing with nothing open still makes an unattached question, which is
     correct: it was not about anything in particular. -->
<CaptureDialog
	bind:open={captureOpen}
	topicId={slotContext === 'canvas' ? (slot.topic?.id ?? null) : null}
	sourceNoteId={slot.noteId}
	topicTitle={slotContext === 'canvas' ? (slot.topic?.title ?? null) : null}
/>
<TopicSearch bind:open={searchOpen} />
