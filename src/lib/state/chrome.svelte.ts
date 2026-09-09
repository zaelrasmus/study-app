/**
 * The top bar's contents, owned by whatever route is on screen.
 *
 * The bar spans the working area between the two sidebars, so it belongs to the
 * centre column — and the centre column is a route. A journal needs a week in
 * its bar, a library needs filters, a board needs a breadcrumb. None of that
 * belongs to the shell, so the shell renders the frame and the route fills it.
 *
 * A route sets `bar` on mount and clears it on teardown. When nothing is set
 * the bar falls back to the breadcrumb, which is the right default: it says
 * where you navigated from.
 */

import type { Snippet } from 'svelte';

class Chrome {
	/** Fills the bar between the back/forward arrows and the window controls. */
	bar = $state<Snippet | null>(null);

	/**
	 * Claims the bar for as long as the calling component is alive.
	 *
	 * Returns the teardown, so a route can hand it straight to `$effect` and
	 * never leave a stale bar behind after navigating away.
	 */
	claim(snippet: Snippet) {
		this.bar = snippet;
		return () => {
			if (this.bar === snippet) this.bar = null;
		};
	}
}

export const chrome = new Chrome();
