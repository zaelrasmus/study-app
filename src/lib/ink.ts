/**
 * Freehand stroke geometry, on top of `perfect-freehand`.
 *
 * We store the raw input points and the settings they were drawn with, never a
 * rasterised outline: that keeps strokes crisp at any zoom and leaves the door
 * open to restyling old ink later.
 */

import getStroke from 'perfect-freehand';
import type { Ink } from './types';

/** Feels like a fine-liner rather than a marker. Tuned for a trackpad. */
export const DEFAULT_INK: Omit<Ink, 'points'> = {
	size: 4,
	thinning: 0.6,
	smoothing: 0.5,
	streamline: 0.5
};

/** Converts an outline polygon into an SVG path with quadratic joins. */
function toPath(outline: number[][]): string {
	if (outline.length === 0) return '';

	const d = outline.reduce<string[]>((acc, [x0, y0], i, arr) => {
		const [x1, y1] = arr[(i + 1) % arr.length];
		acc.push(`${x0.toFixed(2)},${y0.toFixed(2)}`, `${((x0 + x1) / 2).toFixed(2)},${((y0 + y1) / 2).toFixed(2)}`);
		return acc;
	}, []);

	return `M${d[0]} Q${d.slice(1).join(' ')} Z`;
}

export function strokePath(ink: Ink): string {
	const outline = getStroke(ink.points, {
		size: ink.size,
		thinning: ink.thinning,
		smoothing: ink.smoothing,
		streamline: ink.streamline,
		simulatePressure: true
	});

	return toPath(outline);
}

/**
 * Normalises a stroke so its points are relative to its own top-left corner,
 * returning the canvas offset separately. Storing strokes locally means moving
 * one later is a change of position, not a rewrite of every point.
 */
export function normalise(points: [number, number, number][]): {
	points: [number, number, number][];
	x: number;
	y: number;
} {
	const minX = Math.min(...points.map((p) => p[0]));
	const minY = Math.min(...points.map((p) => p[1]));

	return {
		points: points.map(([x, y, p]) => [x - minX, y - minY, p]),
		x: minX,
		y: minY
	};
}
