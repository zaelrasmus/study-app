/**
 * shadcn-svelte's helpers, unmodified.
 *
 * `cn` merges class strings so a component's own Tailwind classes can be
 * overridden by a caller's without fighting over specificity — last one wins,
 * per property. The `Without*` types strip slot props that would otherwise
 * collide when a wrapper forwards everything down.
 *
 * Vendored rather than written here: if the upstream generator changes, this
 * file is expected to be replaced wholesale.
 */

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
	return twMerge(clsx(inputs));
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChild<T> = T extends { child?: any } ? Omit<T, "child"> : T;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type WithoutChildren<T> = T extends { children?: any } ? Omit<T, "children"> : T;
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & { ref?: U | null };
