<script lang="ts">
	/**
	 * Light, dark, or whatever the machine is doing.
	 *
	 * One button rather than a settings screen, for the same reason there are
	 * only three other preferences in the whole app: a settings surface is a
	 * fiddling surface.
	 *
	 * Three states, not two. `System` is where the app starts, so a plain
	 * light/dark toggle would take a choice you never made — the first click
	 * would silently pin the theme and there would be no way back to following
	 * the machine.
	 *
	 * It names its current state instead of only implying it with an icon. A
	 * cycling control whose next stop you have to guess is a small puzzle, and
	 * the sidebar has room for the word.
	 */
	import { userPrefersMode, systemPrefersMode } from 'mode-watcher';
	import SunIcon from '@lucide/svelte/icons/sun';
	import MoonIcon from '@lucide/svelte/icons/moon';
	import MonitorIcon from '@lucide/svelte/icons/monitor';

	type Mode = 'system' | 'light' | 'dark';

	/** System first, so the cycle comes back round to it. */
	const order: Mode[] = ['system', 'light', 'dark'];

	const faces = {
		system: { icon: MonitorIcon, label: 'System' },
		light: { icon: SunIcon, label: 'Light' },
		dark: { icon: MoonIcon, label: 'Dark' }
	};

	const current = $derived((userPrefersMode.current ?? 'system') as Mode);
	const Icon = $derived(faces[current].icon);
	const label = $derived(faces[current].label);

	/** What the machine is actually doing — only meaningful while we follow it. */
	const following = $derived(current === 'system' ? (systemPrefersMode.current ?? '') : '');

	const next = $derived(faces[order[(order.indexOf(current) + 1) % order.length]].label);

	function cycle() {
		userPrefersMode.current = order[(order.indexOf(current) + 1) % order.length];
	}
</script>

<button
	type="button"
	onclick={cycle}
	title="Theme: {label}{following ? ` (${following})` : ''} — click for {next}"
	class="hover:bg-sidebar-accent/60 flex h-[30px] w-full items-center gap-2.5 rounded-md px-2.5 transition-colors"
>
	<Icon class="size-4 shrink-0 opacity-45" />
	<span class="grow truncate text-left text-[12.5px] opacity-65">{label}</span>
	{#if following}
		<!-- Which way "system" is currently pointing, so the word is not a mystery. -->
		<span class="label shrink-0 !text-[8px]">{following}</span>
	{/if}
</button>
