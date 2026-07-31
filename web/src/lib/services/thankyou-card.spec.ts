import { describe, expect, it } from 'vitest';
import {
	CENTRE_FOCAL,
	WIN,
	coupleLine,
	coverScale,
	fileStem,
	focalFromPosition,
	layoutPhoto
} from './thankyou-card';

// A landscape source, cover-fitted into the 296x250 window: draws 333.3x250,
// so there is horizontal slack but none at all vertically.
const LANDSCAPE = { w: 1200, h: 900 };
// A portrait source: draws 296x394.7, slack the other way.
const PORTRAIT = { w: 900, h: 1200 };

describe('coupleLine', () => {
	it('leads with the bride by default', () => {
		expect(coupleLine({ bride: 'Amaya', groom: 'Nuwan', precedence: 'bride' })).toBe(
			'Amaya & Nuwan'
		);
	});

	it('leads with the groom when the event says so', () => {
		expect(coupleLine({ bride: 'Amaya', groom: 'Nuwan', precedence: 'groom' })).toBe(
			'Nuwan & Amaya'
		);
	});

	it('drops a missing name rather than leaving a dangling ampersand', () => {
		expect(coupleLine({ bride: '  ', groom: 'Nuwan', precedence: 'bride' })).toBe('Nuwan');
		expect(coupleLine({ bride: '', groom: '', precedence: 'bride' })).toBe('');
	});
});

describe('fileStem', () => {
	it('keeps non-Latin scripts instead of collapsing them', () => {
		// The old sanitiser stripped these entirely, so every Sinhala guest got
		// the same filename.
		expect(fileStem('චිරත් පවුල')).toBe('චිරත්-පවුල');
		expect(fileStem('ராஜா')).toBe('ராஜா');
	});

	it('hyphenates spaces and strips filesystem-hostile characters', () => {
		expect(fileStem('The Fernando Family')).toBe('The-Fernando-Family');
		expect(fileStem('A/B: "C" <D>.png')).toBe('AB-C-Dpng');
		expect(fileStem('Amaya & Nuwan')).toBe('Amaya-Nuwan');
	});

	it('falls back when nothing survives', () => {
		expect(fileStem('   ')).toBe('card');
		expect(fileStem('...', 'fallback')).toBe('fallback');
		expect(fileStem('', '')).toBe('');
	});
});

describe('coverScale', () => {
	it('picks the axis that needs the most magnification', () => {
		expect(coverScale(LANDSCAPE.w, LANDSCAPE.h)).toBeCloseTo(WIN.h / LANDSCAPE.h, 6);
		expect(coverScale(PORTRAIT.w, PORTRAIT.h)).toBeCloseTo(WIN.w / PORTRAIT.w, 6);
	});
});

describe('layoutPhoto', () => {
	it('centres a cover fit and always fills the window', () => {
		const l = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, 1, CENTRE_FOCAL);
		expect(l.x).toBeCloseTo((WIN.w - LANDSCAPE.w * l.scale) / 2, 6);
		expect(l.y).toBeCloseTo(0, 6);
		// Covered: top-left at or before the origin, bottom-right at or past it.
		expect(l.x).toBeLessThanOrEqual(1e-9);
		expect(l.x + LANDSCAPE.w * l.scale).toBeGreaterThanOrEqual(WIN.w - 1e-9);
		expect(l.y + LANDSCAPE.h * l.scale).toBeGreaterThanOrEqual(WIN.h - 1e-9);
	});

	it('pins the focal point to the window centre so zoom stays anchored', () => {
		// The regression this replaces: scaling about the top-left dragged the
		// subject out of frame as zoom rose.
		for (const zoom of [1, 1.5, 2.5, 4]) {
			const l = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, zoom, CENTRE_FOCAL);
			const centre = focalFromPosition(l.x, l.y, LANDSCAPE.w, LANDSCAPE.h, l.scale);
			expect(centre.x).toBeCloseTo(0.5, 6);
			expect(centre.y).toBeCloseTo(0.5, 6);
		}
	});

	it('clamps a focal point that would uncover the window', () => {
		const l = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, 1, { x: 0, y: 0 });
		expect(l.x).toBeLessThanOrEqual(1e-9);
		expect(l.y).toBeCloseTo(0, 6);
		// No vertical slack at all at zoom 1, so the focal point is pinned mid-axis.
		expect(l.focal.y).toBeCloseTo(0.5, 6);
		// Horizontally it is pushed to the left edge, not past it.
		expect(l.focal.x).toBeCloseTo(WIN.w / 2 / (LANDSCAPE.w * l.scale), 6);
	});

	it('opens up travel on both axes once zoomed in', () => {
		const l = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, 2, { x: 0.1, y: 0.1 });
		expect(l.focal.x).toBeGreaterThan(0);
		expect(l.focal.x).toBeLessThan(0.5);
		expect(l.focal.y).toBeGreaterThan(0);
		expect(l.focal.y).toBeLessThan(0.5);
		expect(l.x + LANDSCAPE.w * l.scale).toBeGreaterThanOrEqual(WIN.w - 1e-9);
		expect(l.y + LANDSCAPE.h * l.scale).toBeGreaterThanOrEqual(WIN.h - 1e-9);
	});

	it('handles a portrait source symmetrically', () => {
		const l = layoutPhoto(PORTRAIT.w, PORTRAIT.h, 1, { x: 0.9, y: 0.9 });
		expect(l.focal.x).toBeCloseTo(0.5, 6);
		expect(l.x).toBeCloseTo(0, 6);
		expect(l.y).toBeLessThanOrEqual(1e-9);
		expect(l.y + PORTRAIT.h * l.scale).toBeGreaterThanOrEqual(WIN.h - 1e-9);
	});

	it('never zooms below the cover fit', () => {
		const under = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, 0.2, CENTRE_FOCAL);
		expect(under.scale).toBeCloseTo(coverScale(LANDSCAPE.w, LANDSCAPE.h), 6);
	});
});

describe('focalFromPosition', () => {
	it('round-trips with layoutPhoto', () => {
		const focal = { x: 0.42, y: 0.58 };
		const l = layoutPhoto(LANDSCAPE.w, LANDSCAPE.h, 3, focal);
		const back = focalFromPosition(l.x, l.y, LANDSCAPE.w, LANDSCAPE.h, l.scale);
		expect(back.x).toBeCloseTo(l.focal.x, 6);
		expect(back.y).toBeCloseTo(l.focal.y, 6);
	});
});
