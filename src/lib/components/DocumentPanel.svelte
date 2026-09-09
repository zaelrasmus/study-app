<script lang="ts">
	/**
	 * The assembled document.
	 *
	 * Read-only, and deliberately without a single styling control: no themes, no
	 * fonts, no layout options, no export options. The stated failure mode is
	 * spending the effort on making it look publishable, so there is nowhere for
	 * that effort to go. One typography, non-negotiable.
	 *
	 * The unit of completion is the frame, not the topic. Each section has visible
	 * edges and can actually be finished; the topic-level document is only ever
	 * their concatenation, and is never a task in itself.
	 */
	import * as Sheet from '$lib/components/ui/sheet';
	import * as ipc from '$lib/ipc';
	import type { DocumentSection } from '$lib/types';
	import StateMark from '$lib/components/canvas/StateMark.svelte';
	// The same stylesheet the editor uses, so a heading, list or quote looks
	// identical here and in the note it came from.
	import '$lib/components/edra/shadcn/editor.css';

	let { open = $bindable(false), topicId }: { open?: boolean; topicId: string } = $props();

	let sections = $state<DocumentSection[]>([]);

	$effect(() => {
		if (open) ipc.topicDocument(topicId).then((s) => (sections = s));
	});

	const written = $derived(sections.filter((s) => s.notes.length > 0));
	const words = $derived(
		written.reduce(
			(total, s) =>
				total + s.notes.reduce((n, note) => n + note.body_text.trim().split(/\s+/).filter(Boolean).length, 0),
			0
		)
	);
</script>

<Sheet.Root bind:open>
	<Sheet.Content side="right" class="w-full gap-0 p-0 sm:max-w-2xl">
		<Sheet.Header class="border-b px-10 py-3" style="border-color: var(--hairline)">
			<Sheet.Title class="label">Document</Sheet.Title>
			<Sheet.Description class="font-mono text-[10px] tracking-wider opacity-50">
				{written.length} section{written.length === 1 ? '' : 's'} · {words} words · assembled from the frames
			</Sheet.Description>
		</Sheet.Header>

		<div class="selectable min-h-0 grow overflow-y-auto px-10 py-10">
			{#if sections.length === 0}
				<p class="prose-doc text-muted-foreground max-w-prose">
					No frames yet. Press <kbd class="font-mono text-[11px]">F</kbd> and drag one around a
					cluster of notes — that is the whole act of writing this.
				</p>
			{:else if written.length === 0}
				<p class="prose-doc text-muted-foreground max-w-prose">
					The frames exist but nothing has been put in them. Drag a note inside a frame to
					commit it to the document.
				</p>
			{:else}
				<article class="prose-doc flex max-w-prose flex-col gap-12">
					{#each written as section (section.frame.id)}
						<section class="flex flex-col gap-3">
							<header class="flex items-baseline gap-2.5">
								<span class="font-mono text-[10px] opacity-30 tabular-nums">
									{String(section.frame.order_index + 1).padStart(2, '0')}
								</span>
								<h2 class="text-[17px]">
									{section.frame.label || 'Untitled section'}
								</h2>
							</header>

							{#each section.notes as note (note.id)}
								<div class="flex flex-col gap-1">
									{#if note.title}
										<h3 class="flex items-center gap-2 font-sans text-[12.5px] font-medium">
											<StateMark
										state={note.state}
										visible={note.visible}
										neverContrasted={note.never_contrasted}
										class="opacity-70"
									/>
											{note.title}
										</h3>
									{/if}

									{#if note.body_html.trim()}
										<!-- Rendered rich text. The `tiptap` class is what Edra's
										     stylesheet targets, so headings, lists, quotes and code
										     all keep the formatting they were written with.

										     `@html` is safe here in the way that matters: this is a
										     local, single-user app and the only author of this
										     markup is the person reading it. -->
										<div class="tiptap">{@html note.body_html}</div>
									{:else if note.body_text.trim()}
										<!-- Written before rich text was stored: show it as-is
										     rather than pretending the note is empty. -->
										<p class="whitespace-pre-wrap">{note.body_text}</p>
									{/if}
								</div>
							{/each}
						</section>
					{/each}
				</article>
			{/if}
		</div>
	</Sheet.Content>
</Sheet.Root>
