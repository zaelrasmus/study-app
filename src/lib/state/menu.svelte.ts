/**
 * The right-click menu.
 *
 * One menu for the whole app, opened with a list of items rather than each
 * surface building its own — so every context menu looks and behaves the same,
 * and destructive items are marked in one place instead of six.
 */

export interface MenuItem {
	label: string;
	/** Marks the item as irreversible. Rendered apart and in the action colour. */
	destructive?: boolean;
	/** A row of colour swatches instead of a label. */
	swatches?: { id: string | null; label: string }[];
	onpick?: () => void;
	onswatch?: (id: string | null) => void;
}

class Menu {
	open = $state(false);
	x = $state(0);
	y = $state(0);
	items = $state<MenuItem[]>([]);

	show(event: MouseEvent, items: MenuItem[]) {
		event.preventDefault();
		event.stopPropagation();

		// Kept inside the window: a menu opened near the right edge that renders
		// off-screen is a menu you cannot use.
		this.x = Math.min(event.clientX, window.innerWidth - 220);
		this.y = Math.min(event.clientY, window.innerHeight - 40 - items.length * 30);
		this.items = items;
		this.open = true;
	}

	close() {
		this.open = false;
		this.items = [];
	}
}

export const menu = new Menu();
