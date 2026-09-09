/**
 * The card palette.
 *
 * A *chosen* colour, not an encoded one — which is why it lives in a different
 * visual channel from knowledge state. These are pale fills, like tinted paper.
 * State stays a thin saturated rule down the leading edge and a mark beside the
 * title, so a card can be butter-yellow and recalled at the same time and both
 * remain legible.
 *
 * The hues deliberately avoid the two state hues, so a colour can never be
 * mistaken for a signal about what you know.
 */
export type CardColour = 'butter' | 'rose' | 'violet' | 'moss' | 'slate' | 'clay';

export const CARD_COLOURS: { id: CardColour; label: string }[] = [
	{ id: 'butter', label: 'Butter' },
	{ id: 'clay', label: 'Clay' },
	{ id: 'rose', label: 'Rose' },
	{ id: 'moss', label: 'Moss' },
	{ id: 'slate', label: 'Slate' },
	{ id: 'violet', label: 'Violet' }
];

export function fill(colour: string | null): string | null {
	if (!colour) return null;
	return CARD_COLOURS.some((c) => c.id === colour) ? `var(--card-${colour})` : null;
}
