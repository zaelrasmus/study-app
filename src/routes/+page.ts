import { redirect } from '@sveltejs/kit';
import { prefs } from '$lib/state/prefs.svelte';

/**
 * The front door, which is not a screen.
 *
 * Opening the app used to land on the question queue, which is a list of things
 * you have not answered yet — a fine place to *go*, and the wrong thing to be
 * handed before you have written a word. The day's page is where capture starts,
 * so that is where the app opens.
 *
 * `/` is now nothing but this choice, which is what makes it configurable: the
 * queue kept its own address at `/questions`, so nothing had to be traded away
 * to move the landing.
 */
export const load = () => {
	redirect(307, prefs.startPath);
};
