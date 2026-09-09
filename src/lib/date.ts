/**
 * Local-calendar dates.
 *
 * Everything here works in the user's own timezone and formats as `YYYY-MM-DD`,
 * which is what the Rust side stores and compares. Deliberately not `toISOString`:
 * that converts to UTC first, so late-evening captures would land on tomorrow.
 */

export function iso(d: Date): string {
	const month = String(d.getMonth() + 1).padStart(2, '0');
	const day = String(d.getDate()).padStart(2, '0');
	return `${d.getFullYear()}-${month}-${day}`;
}

export function addDays(from: Date, days: number): Date {
	const d = new Date(from);
	d.setDate(d.getDate() + days);
	return d;
}

/**
 * How many days past the selected one the journal strip shows.
 *
 * Enough to orient you, not enough to be a planner: you can see what is coming
 * but you still cannot write into it.
 */
export const FUTURE_DAYS = 3;

/** Monday-first, like the calendar in every Spanish-speaking locale. */
export function weekStart(from: Date): Date {
	const d = new Date(from);
	d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
	d.setHours(0, 0, 0, 0);
	return d;
}

export const today = () => iso(new Date());
