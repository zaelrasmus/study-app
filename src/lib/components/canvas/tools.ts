/** Canvas tools. Kept in its own module so the toolbar and the canvas agree. */
export type Tool = 'select' | 'draw' | 'frame' | 'text' | 'connect';

export const TOOL_HINT: Record<Tool, string> = {
	select:
		'Double-click empty space for a note. Drop an image or a PDF anywhere. Drag a note into a frame to commit it.',
	draw: 'Draw freely. Esc to stop.',
	frame: 'Drag a rectangle to make a section. Esc to cancel.',
	text: 'Click anywhere to write. It is loose text until you make it a note.',
	connect: 'Drag from one card to another to point at it. Click an arrow to drop it.'
};
