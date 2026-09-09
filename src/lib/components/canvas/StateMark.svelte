<script lang="ts">
	/**
	 * The memory axis, in one 10px mark — or nothing at all.
	 *
	 * A note that has never been reproduced from memory renders **nothing**. Not
	 * a hollow ring, not a grey word: nothing. `raw` and `draft` are absences of
	 * an attempt, not stages of failure, and labelling a note you only wanted to
	 * write with how far it is from the end of a pipeline is the whole complaint
	 * this component exists to answer.
	 *
	 * What survives is exactly the set that carries colour, which is why the
	 * palette can never read as a grade: there is no scale, only "you did this"
	 * and "there is something to do".
	 */
	import type { RecallState } from '$lib/types';
	import { STATE_LABEL, STATE_MEANING } from '$lib/types';

	let {
		state,
		visible = true,
		neverContrasted = false,
		class: className = ''
	}: {
		state: RecallState;
		/** Computed in Rust. False for anything that never attempted memory. */
		visible?: boolean;
		neverContrasted?: boolean;
		class?: string;
	} = $props();

	const colour = $derived(
		state === 'recalled' ? 'var(--state-recalled)' : 'var(--state-action)'
	);

	const title = $derived(
		`${STATE_LABEL[state]} — ${STATE_MEANING[state]}` +
			(neverContrasted ? ' Never contrasted against a source.' : '')
	);
</script>

{#if visible}
	<span class="inline-flex shrink-0 items-center gap-[3px] {className}" {title}>
		<svg viewBox="0 0 10 10" class="size-2.5" style="color: {colour}" aria-hidden="true">
			{#if state === 'recalled'}
				<!-- Reproduced blind, unchanged since. -->
				<circle cx="5" cy="5" r="3.5" fill="currentColor" />
			{:else if state === 'edited_since_recall'}
				<!-- The text moved after the recall. -->
				<circle cx="5" cy="5" r="3.5" fill="none" stroke="currentColor" stroke-width="1.5" />
				<path d="M2.6 6.4 L7.4 3.6" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
			{:else if state === 'failing'}
				<!-- Recalled once, then missed in review. -->
				<circle cx="5" cy="5" r="3.5" fill="currentColor" opacity="0.3" />
				<circle cx="5" cy="5" r="3.5" fill="none" stroke="currentColor" stroke-width="1.5" />
			{/if}
		</svg>

		{#if neverContrasted}
			<!-- Axis 2, deliberately tiny and uncoloured: a note about what the app
			     does not know, not a problem to be nagged about. -->
			<span
				class="size-[3px] rounded-full opacity-45"
				style="background: currentColor"
				aria-hidden="true"
			></span>
		{/if}
	</span>
{/if}

