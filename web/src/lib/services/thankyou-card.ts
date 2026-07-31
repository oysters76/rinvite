// Client-side thank-you card editor, built on Konva. Renders a 4x6 (4R) floral
// card into a live-preview stage and exports it as a print-ready PNG — the same
// stage is the single source of truth for both the preview and the export, so
// "what you see is what you get". No backend involved.
//
// The floral/gold look mirrors the printed invite and the e-invite (see
// assets/pdf-config.json and assets/einvite/template.html): ivory paper, gilded
// peonies, gold gradient script for the couple, EB Garamond for the prose. The
// peony construction below is a port of flowerSVG() in the e-invite template, so
// the card a guest receives matches the invite they already opened.
//
// Konva is imported lazily inside createCardEditor() — the events page links to
// this module, and there is no reason to ship a canvas library to someone who
// never opens the dialog.

import type { Precedence } from '$lib/api';
import type { Stage } from 'konva/lib/Stage';
import type { Layer } from 'konva/lib/Layer';
import type { Image as KonvaImage } from 'konva/lib/shapes/Image';
import type { Text as KonvaText } from 'konva/lib/shapes/Text';

// ---------------------------------------------------------------------------
// Geometry. Logical stage is 400x600 (a 4:6 / 4R ratio). Exporting at
// pixelRatio 3 yields 1200x1800 px = 4x6 in @ 300 DPI.
// ---------------------------------------------------------------------------
export const CARD_W = 400;
export const CARD_H = 600;
export const EXPORT_PIXEL_RATIO = 3; // 400x600 * 3 = 1200x1800

/** Photo window (framed opening near the top of the card). */
export const WIN = { x: 52, y: 92, w: 296, h: 250 };

/** Vertical band the message may occupy before it would reach the date line. */
const MSG_TOP = 458;
const MSG_BAND_H = 80;
/** Kept clear of the bottom corner sprays. */
const MSG_X = 68;
/** Message shrinks a step at a time rather than overflowing. */
const MSG_SIZES = [14.5, 13.5, 12.5, 11.5];
const GUEST_SIZES = [19, 17, 15];

export const MAX_GUEST_LEN = 60;
export const MAX_MESSAGE_LEN = 180;

export const DEFAULT_MESSAGE =
	'Thank you for celebrating with us. Your presence made our day complete.';

// ---------------------------------------------------------------------------
// Palette — lifted from the invite's gold/ivory theme (template.html :root).
// ---------------------------------------------------------------------------
const IVORY = '#fbf7ee';
const CREAM = '#f3ead6';
const INK = '#3b3326';
const SOFT_INK = '#6b5d45';
const GOLD_LINE = '#c8a44d';
const GOLD_DEEP = '#a9791c';
const GOLD_MID = '#b38728';
// Gradient stops for gilt script, matching --gold-text in the e-invite.
const GOLD_TEXT_STOPS = [
	0,
	'#7a5712',
	0.26,
	'#b98b22',
	0.52,
	'#8a6414',
	0.76,
	'#bd9233',
	1,
	'#6b4d0f'
];

// Family names registered by ensureCardFonts()…
const FAM_SCRIPT = 'RinviteGreatVibes';
const FAM_SERIF = 'RinviteEBGaramond';
const FAM_CAPS = 'RinviteCinzel';
// …and the stacks the canvas draws with. The generic fallback matters: none of
// these faces cover Sinhala or Tamil, and a guest's name must render as their
// name rather than as tofu.
const F_SCRIPT = `${FAM_SCRIPT}, serif`;
const F_SERIF = `${FAM_SERIF}, serif`;
const F_CAPS = `${FAM_CAPS}, serif`;

export interface CardModel {
	bride: string;
	groom: string;
	/** Which name leads, mirroring the event's own setting. */
	precedence: Precedence;
	guest: string;
	message: string;
	/** Already human-formatted (e.g. "September 25, 2026"); rendered verbatim. */
	date: string;
}

/** The couple line, ordered by precedence. Pure, so it can be unit-tested. */
export function coupleLine(m: Pick<CardModel, 'bride' | 'groom' | 'precedence'>): string {
	const bride = m.bride.trim();
	const groom = m.groom.trim();
	const ordered = m.precedence === 'groom' ? [groom, bride] : [bride, groom];
	return ordered.filter(Boolean).join(' & ');
}

/**
 * Turn a name into a safe filename stem without destroying non-Latin scripts —
 * a Sinhala or Tamil guest name must not collapse to nothing.
 */
export function fileStem(raw: string, fallback = 'card'): string {
	const safe = raw
		.normalize('NFC')
		// Strip only what a filesystem objects to; keep letters of any script.
		.replace(/[\\/?%*:|"<>.#&]/g, '')
		.replace(/\s+/g, ' ')
		.trim()
		.replace(/ /g, '-');
	return safe || fallback;
}

// ---------------------------------------------------------------------------
// Photo framing. The photo is positioned by a *focal point*: the normalised
// point of the source image that sits at the centre of the window. That makes
// zoom naturally centre-anchored and keeps the "must stay covered" clamp in one
// pure, testable place.
// ---------------------------------------------------------------------------
export interface Focal {
	x: number;
	y: number;
}

export interface PhotoLayout {
	/** Group-local position for the image node. */
	x: number;
	y: number;
	scale: number;
	/** The focal point after clamping — feed this back into your state. */
	focal: Focal;
}

const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

export const CENTRE_FOCAL: Focal = { x: 0.5, y: 0.5 };

/** Cover-fit scale: the smallest scale at which the image still fills the window. */
export const coverScale = (imgW: number, imgH: number, win = WIN): number =>
	Math.max(win.w / imgW, win.h / imgH);

export function layoutPhoto(
	imgW: number,
	imgH: number,
	zoom: number,
	focal: Focal,
	win = WIN
): PhotoLayout {
	const scale = coverScale(imgW, imgH, win) * Math.max(1, zoom);
	const drawnW = imgW * scale;
	const drawnH = imgH * scale;
	// Half a window, expressed as a fraction of the drawn image. The focal point
	// cannot come closer than this to an edge without uncovering the window.
	const marginX = Math.min(win.w / 2 / drawnW, 0.5);
	const marginY = Math.min(win.h / 2 / drawnH, 0.5);
	const fx = clamp(focal.x, marginX, 1 - marginX);
	const fy = clamp(focal.y, marginY, 1 - marginY);
	return {
		x: win.w / 2 - fx * drawnW,
		y: win.h / 2 - fy * drawnH,
		scale,
		focal: { x: fx, y: fy }
	};
}

/** Inverse of layoutPhoto: recover the focal point from a dragged position. */
export function focalFromPosition(
	x: number,
	y: number,
	imgW: number,
	imgH: number,
	scale: number,
	win = WIN
): Focal {
	return {
		x: (win.w / 2 - x) / (imgW * scale),
		y: (win.h / 2 - y) / (imgH * scale)
	};
}

// ---------------------------------------------------------------------------
// Fonts. Loaded once from /static/fonts via the FontFace API so Konva measures
// and rasterises with the real faces (canvas text needs the fonts resident).
// These mirror assets/fonts/, which the server-side PDF pipeline uses.
// ---------------------------------------------------------------------------
let fontsPromise: Promise<void> | null = null;

export function ensureCardFonts(): Promise<void> {
	if (fontsPromise) return fontsPromise;
	const faces: FontFace[] = [
		new FontFace(FAM_SCRIPT, "url('/fonts/GreatVibes-Regular.ttf')"),
		new FontFace(FAM_SERIF, "url('/fonts/EBGaramond-Regular.ttf')"),
		new FontFace(FAM_SERIF, "url('/fonts/EBGaramond-Italic.ttf')", { style: 'italic' }),
		new FontFace(FAM_CAPS, "url('/fonts/Cinzel-Variable.ttf')")
	];
	fontsPromise = Promise.all(
		faces.map(async (f) => {
			const loaded = await f.load();
			document.fonts.add(loaded);
		})
	).then(() => undefined);
	return fontsPromise;
}

// ---------------------------------------------------------------------------
// Ornaments. A port of flowerSVG()/petalSVG() from assets/einvite/template.html:
// a gilded peony built from three rings of rotated petals over a radial wash,
// plus loose petals, buds and leaves for the sprays.
//
// Each peony needs its own gradient id — several are inlined into one document.
// ---------------------------------------------------------------------------

/**
 * A peony centred on (0,0) in its own local space, drawn at `size` px across.
 *
 * The e-invite draws these 300px wide over a photographic backdrop; on ivory
 * paper at a third of that they need heavier linework to read at all, so every
 * stroke width is divided by the scale factor to keep a constant on-card weight.
 */
function peony(id: string, size: number, rotate = 0, opacity = 1): string {
	const k = size / 240;
	/** Local stroke width that renders as `px` on the card. */
	const sw = (px: number) => (px / k).toFixed(2);
	let outer = '';
	let mid = '';
	let inner = '';
	for (let i = 0; i < 8; i++)
		outer += `<path d="M120 120 C86 90 82 40 120 20 C158 40 154 90 120 120Z" transform="rotate(${i * 45} 120 120)"/>`;
	for (let i = 0; i < 8; i++)
		mid += `<path d="M120 120 C99 99 96 62 120 48 C144 62 141 99 120 120Z" transform="rotate(${i * 45 + 22.5} 120 120)"/>`;
	for (let i = 0; i < 6; i++)
		inner += `<path d="M120 120 C108 108 106 88 120 80 C134 88 132 108 120 120Z" transform="rotate(${i * 60} 120 120)"/>`;
	let dots = '';
	const dotR = (1.1 / k).toFixed(2);
	for (let i = 0; i < 12; i++) {
		const a = (i / 12) * Math.PI * 2;
		dots += `<circle cx="${(120 + Math.cos(a) * 9).toFixed(2)}" cy="${(120 + Math.sin(a) * 9).toFixed(2)}" r="${dotR}" fill="${GOLD_MID}"/>`;
	}
	return `<g transform="rotate(${rotate}) scale(${k.toFixed(4)}) translate(-120,-120)" opacity="${opacity}">
    <g fill="url(#${id})" stroke="${GOLD_DEEP}" stroke-width="${sw(1.05)}" stroke-linejoin="round">${outer}</g>
    <g fill="#fdf3dd" stroke="${GOLD_MID}" stroke-width="${sw(0.85)}" stroke-linejoin="round">${mid}</g>
    <g fill="#f7ead0" stroke="${GOLD_DEEP}" stroke-width="${sw(0.7)}">${inner}</g>
    <circle cx="120" cy="120" r="14" fill="#f2dfae" stroke="${GOLD_DEEP}" stroke-width="${sw(0.8)}"/>
    ${dots}
  </g>`;
}

// Ends deeper than the e-invite's #f4e7c8 — against ivory paper the original
// wash left the petals almost invisible.
const peonyGradient = (id: string) =>
	`<radialGradient id="${id}" cx="50%" cy="38%" r="65%">
    <stop offset="0%" stop-color="#fffdf7"/><stop offset="55%" stop-color="#fdf2da"/><stop offset="100%" stop-color="#eeddb4"/>
  </radialGradient>`;

/** A single loose petal, centred on (0,0). Ported from petalSVG(). */
function petal(size: number, rotate = 0, opacity = 1): string {
	const k = size / 40;
	return `<g transform="rotate(${rotate}) scale(${k.toFixed(4)}) translate(-20,-25)" opacity="${opacity}">
    <path d="M20 2 C34 12 36 34 20 48 C4 34 6 12 20 2Z" fill="#fffaf0" stroke="${GOLD_LINE}" stroke-width="1"/>
    <path d="M20 6 C20 20 20 34 20 46" stroke="#e3cf95" stroke-width="1" fill="none"/>
  </g>`;
}

/** A narrow leaf — the petal silhouette squeezed, with a centre vein. */
function leaf(size: number, rotate = 0, opacity = 1): string {
	const k = size / 40;
	return `<g transform="rotate(${rotate}) scale(${(k * 0.62).toFixed(4)},${k.toFixed(4)}) translate(-20,-25)" opacity="${opacity}">
    <path d="M20 2 C34 12 36 34 20 48 C4 34 6 12 20 2Z" fill="#f6efdc" stroke="${GOLD_DEEP}" stroke-width="1.4"/>
    <path d="M20 6 C20 20 20 34 20 46" stroke="${GOLD_DEEP}" stroke-width="1.1" fill="none"/>
  </g>`;
}

/** A closed bud on a short stem. */
function bud(size: number, rotate = 0): string {
	const k = size / 40;
	return `<g transform="rotate(${rotate}) scale(${k.toFixed(4)})">
    <path d="M0 -14 C7 -8 8 4 0 12 C-8 4 -7 -8 0 -14Z" fill="#fdf6e6" stroke="${GOLD_LINE}" stroke-width="1.2"/>
    <path d="M0 12 C-1 20 -1 26 0 32" stroke="${GOLD_DEEP}" stroke-width="1.2" fill="none" stroke-linecap="round"/>
  </g>`;
}

/**
 * One corner spray: a cluster that hugs the corner and runs a short way along
 * each border. It is kept inside roughly 100x100 on purpose — the photo window
 * opens at (52, 92), and an earlier, longer spray sent vines and leaves slicing
 * across the middle of the photograph.
 *
 * Deliberately asymmetric so the four mirrored copies read as a composition
 * rather than four identical stamps.
 */
function cornerSpray(idPrefix: string): string {
	return `<g>
    <path d="M4 92 C 8 66, 20 42, 42 26" fill="none" stroke="${GOLD_MID}" stroke-width="1.4" stroke-linecap="round"/>
    <path d="M92 4 C 66 8, 42 20, 26 42" fill="none" stroke="${GOLD_LINE}" stroke-width="1" stroke-linecap="round" opacity="0.85"/>
    <g transform="translate(62,54)">${leaf(26, 42)}</g>
    <g transform="translate(50,70)">${leaf(21, 68, 0.9)}</g>
    <g transform="translate(84,42)">${bud(17, 46)}</g>
    <g transform="translate(38,88)">${bud(15, -54)}</g>
    <g transform="translate(70,74)">${petal(15, -24, 0.9)}</g>
    <g transform="translate(34,34)">${peony(`${idPrefix}a`, 76, -14)}</g>
    <g transform="translate(76,20)">${peony(`${idPrefix}b`, 40, 22, 0.97)}</g>
    <g transform="translate(18,74)">${peony(`${idPrefix}c`, 34, -34, 0.94)}</g>
  </g>`;
}

/** Slim horizontal flourish used between the couple line and the guest line. */
function divider(y: number, halfWidth: number): string {
	const cx = CARD_W / 2;
	return `<g transform="translate(${cx},${y})" fill="none" stroke="${GOLD_LINE}" stroke-width="1" stroke-linecap="round">
    <path d="M${-halfWidth} 0 C ${-halfWidth * 0.45} -5, ${-18} -5, -11 0"/>
    <path d="M${halfWidth} 0 C ${halfWidth * 0.45} -5, ${18} -5, 11 0"/>
    <path d="M-11 0 C -7 -6, -3 -7, 0 -7 C 3 -7, 7 -6, 11 0 C 7 6, 3 7, 0 7 C -3 7, -7 6, -11 0Z" fill="#fdf6e6" stroke="${GOLD_DEEP}"/>
    <circle cx="0" cy="0" r="2" fill="${GOLD_MID}" stroke="none"/>
  </g>`;
}

/** Small closing motif under the date. */
function baseFlourish(y: number): string {
	const cx = CARD_W / 2;
	return `<g transform="translate(${cx},${y})" fill="none" stroke="${GOLD_LINE}" stroke-width="1" stroke-linecap="round">
    <path d="M-30 0 C -20 -6, -10 -6, 0 0 C 10 -6, 20 -6, 30 0"/>
    <circle cx="0" cy="3.5" r="1.8" fill="${GOLD_MID}" stroke="none"/>
    <circle cx="-13" cy="1.5" r="1.2" fill="${GOLD_MID}" stroke="none"/>
    <circle cx="13" cy="1.5" r="1.2" fill="${GOLD_MID}" stroke="none"/>
  </g>`;
}

/**
 * The whole static decoration: double border, four corner sprays, the window
 * hairline and the two flourishes.
 *
 * Emitted at export resolution (viewBox stays 400x600) so the 3x PNG keeps
 * crisp gilt linework — the browser downsamples it for the preview. Rendering
 * it at 400x600 and letting the export upscale is what made the earlier "300
 * DPI" output soft.
 */
function frameSvg(): string {
	const w = CARD_W * EXPORT_PIXEL_RATIO;
	const h = CARD_H * EXPORT_PIXEL_RATIO;
	// Bottom corners are scaled back so the composition stays top-weighted.
	const corners = [
		`<g transform="translate(14,14)">${cornerSpray('tl')}</g>`,
		`<g transform="translate(${CARD_W - 14},14) scale(-1,1)">${cornerSpray('tr')}</g>`,
		`<g transform="translate(16,${CARD_H - 16}) scale(0.7,-0.7)">${cornerSpray('bl')}</g>`,
		`<g transform="translate(${CARD_W - 16},${CARD_H - 16}) scale(-0.7,-0.7)">${cornerSpray('br')}</g>`
	].join('');
	const gradients = ['tl', 'tr', 'bl', 'br']
		.flatMap((p) => ['a', 'b', 'c'].map((s) => peonyGradient(`${p}${s}`)))
		.join('');
	return `
<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${CARD_W} ${CARD_H}">
  <defs>${gradients}</defs>
  <rect x="16" y="16" width="${CARD_W - 32}" height="${CARD_H - 32}" fill="none" stroke="${GOLD_LINE}" stroke-width="2.5"/>
  <rect x="22" y="22" width="${CARD_W - 44}" height="${CARD_H - 44}" fill="none" stroke="${GOLD_DEEP}" stroke-width="1"/>
  <rect x="${WIN.x}" y="${WIN.y}" width="${WIN.w}" height="${WIN.h}" fill="none" stroke="${GOLD_LINE}" stroke-width="1.5"/>
  <rect x="${WIN.x - 4}" y="${WIN.y - 4}" width="${WIN.w + 8}" height="${WIN.h + 8}" fill="none" stroke="${GOLD_DEEP}" stroke-width="0.8" opacity="0.7"/>
  ${divider(408, 84)}
  ${baseFlourish(570)}
  ${corners}
</svg>`;
}

function loadImage(src: string): Promise<HTMLImageElement> {
	return new Promise((resolve, reject) => {
		const img = new Image();
		img.onload = () => resolve(img);
		img.onerror = () => reject(new Error('image load failed'));
		img.src = src;
	});
}

// ---------------------------------------------------------------------------
// Editor controller. Owns the Konva stage and exposes a small imperative API
// for the Svelte component to drive.
// ---------------------------------------------------------------------------
export interface CardEditor {
	setPhoto(img: HTMLImageElement): void;
	hasPhoto(): boolean;
	setZoom(zoom: number): void;
	/** Shift the framing by a fraction of the window (keyboard nudge). */
	nudge(dx: number, dy: number): void;
	/** Back to a centred cover fit at zoom 1. */
	resetFraming(): void;
	update(model: CardModel): void;
	toBlob(): Promise<Blob>;
	destroy(): void;
}

export async function createCardEditor(container: HTMLDivElement): Promise<CardEditor> {
	// Lazy so Konva stays out of the events-page chunk.
	const [{ default: Konva }] = await Promise.all([import('konva'), ensureCardFonts()]);

	const stage: Stage = new Konva.Stage({ container, width: CARD_W, height: CARD_H });
	const layer: Layer = new Konva.Layer();
	stage.add(layer);

	// Paper background + cream inner panel.
	layer.add(new Konva.Rect({ x: 0, y: 0, width: CARD_W, height: CARD_H, fill: IVORY }));
	layer.add(
		new Konva.Rect({
			x: 8,
			y: 8,
			width: CARD_W - 16,
			height: CARD_H - 16,
			fill: CREAM,
			opacity: 0.35,
			listening: false
		})
	);

	// Photo window: a clipped group so the (draggable) image never spills out.
	const photoGroup = new Konva.Group({
		x: WIN.x,
		y: WIN.y,
		clipX: 0,
		clipY: 0,
		clipWidth: WIN.w,
		clipHeight: WIN.h
	});
	layer.add(photoGroup);

	// Placeholder shown until a photo is uploaded.
	const placeholder = new Konva.Rect({
		x: 0,
		y: 0,
		width: WIN.w,
		height: WIN.h,
		fill: '#efe7d5',
		stroke: GOLD_LINE,
		strokeWidth: 1,
		dash: [6, 5],
		listening: false
	});
	const placeholderText = new Konva.Text({
		x: 0,
		y: WIN.h / 2 - 10,
		width: WIN.w,
		align: 'center',
		text: 'Upload a photo',
		fontFamily: F_SERIF,
		fontStyle: 'italic',
		fontSize: 16,
		fill: SOFT_INK,
		listening: false
	});
	photoGroup.add(placeholder, placeholderText);

	// Static decoration, including the hairline around the window opening. This
	// is drawn above the photo, so it MUST NOT listen: Konva's hit canvas fills a
	// shape's path with its colour key even when the scene shape has no fill, so
	// a listening overlay here swallows every drag on the photo beneath it.
	const frameImg = await loadImage('data:image/svg+xml;utf8,' + encodeURIComponent(frameSvg()));
	layer.add(
		new Konva.Image({
			x: 0,
			y: 0,
			width: CARD_W,
			height: CARD_H,
			image: frameImg,
			listening: false
		})
	);

	// --- Text nodes ---------------------------------------------------------
	const gold = (fontSize: number) => ({
		fillLinearGradientStartPoint: { x: 0, y: 0 },
		fillLinearGradientEndPoint: { x: 0, y: fontSize },
		fillLinearGradientColorStops: GOLD_TEXT_STOPS
	});

	// Fixed "Thank You" script heading.
	layer.add(
		new Konva.Text({
			x: 0,
			y: 40,
			width: CARD_W,
			align: 'center',
			text: 'Thank You',
			fontFamily: F_SCRIPT,
			fontSize: 44,
			listening: false,
			...gold(44)
		})
	);

	const couple: KonvaText = new Konva.Text({
		x: 0,
		y: 356,
		width: CARD_W,
		align: 'center',
		text: '',
		fontFamily: F_SCRIPT,
		fontSize: 34,
		listening: false,
		...gold(34)
	});
	const guestText: KonvaText = new Konva.Text({
		x: 20,
		y: 428,
		width: CARD_W - 40,
		align: 'center',
		text: '',
		fontFamily: F_SERIF,
		fontStyle: 'italic',
		fontSize: GUEST_SIZES[0],
		fill: INK,
		listening: false
	});
	const messageText: KonvaText = new Konva.Text({
		x: MSG_X,
		y: MSG_TOP,
		width: CARD_W - MSG_X * 2,
		align: 'center',
		text: '',
		fontFamily: F_SERIF,
		fontStyle: 'italic',
		fontSize: MSG_SIZES[0],
		lineHeight: 1.35,
		fill: SOFT_INK,
		listening: false
	});
	const dateText: KonvaText = new Konva.Text({
		x: 0,
		y: 548,
		width: CARD_W,
		align: 'center',
		text: '',
		fontFamily: F_CAPS,
		fontSize: 12.5,
		letterSpacing: 2,
		listening: false,
		...gold(12.5)
	});
	layer.add(couple, guestText, messageText, dateText);

	// --- Photo state --------------------------------------------------------
	let photoNode: KonvaImage | null = null;
	let zoom = 1;
	let focal: Focal = { ...CENTRE_FOCAL };
	let imgW = 0;
	let imgH = 0;

	const currentScale = () => coverScale(imgW, imgH) * zoom;

	function applyLayout() {
		if (!photoNode) return;
		const l = layoutPhoto(imgW, imgH, zoom, focal);
		focal = l.focal;
		photoNode.position({ x: l.x, y: l.y });
		photoNode.scale({ x: l.scale, y: l.scale });
		layer.batchDraw();
	}

	const setCursor = (c: string) => {
		stage.container().style.cursor = c;
	};

	function setPhoto(img: HTMLImageElement) {
		placeholder.hide();
		placeholderText.hide();
		if (photoNode) {
			photoNode.destroy();
			photoNode = null;
		}
		imgW = img.naturalWidth || img.width;
		imgH = img.naturalHeight || img.height;
		if (!imgW || !imgH) return;
		zoom = 1;
		focal = { ...CENTRE_FOCAL };
		const l = layoutPhoto(imgW, imgH, zoom, focal);
		focal = l.focal;
		photoNode = new Konva.Image({
			image: img,
			x: l.x,
			y: l.y,
			scaleX: l.scale,
			scaleY: l.scale,
			draggable: true,
			dragBoundFunc(pos) {
				// pos is absolute (stage) space; convert to group-local, re-derive the
				// focal point, and let layoutPhoto do the covering clamp.
				const next = focalFromPosition(pos.x - WIN.x, pos.y - WIN.y, imgW, imgH, currentScale());
				const clamped = layoutPhoto(imgW, imgH, zoom, next);
				focal = clamped.focal;
				return { x: WIN.x + clamped.x, y: WIN.y + clamped.y };
			}
		});
		photoNode.on('mouseenter', () => setCursor('grab'));
		photoNode.on('mouseleave', () => setCursor('default'));
		photoNode.on('dragstart', () => setCursor('grabbing'));
		photoNode.on('dragend', () => setCursor('grab'));
		photoGroup.add(photoNode);
		photoNode.moveToBottom();
		layer.batchDraw();
	}

	function setZoom(z: number) {
		zoom = Math.max(1, z);
		applyLayout();
	}

	function nudge(dx: number, dy: number) {
		if (!photoNode) return;
		const s = currentScale();
		focal = {
			x: focal.x + (dx * WIN.w) / (imgW * s),
			y: focal.y + (dy * WIN.h) / (imgH * s)
		};
		applyLayout();
	}

	function resetFraming() {
		zoom = 1;
		focal = { ...CENTRE_FOCAL };
		applyLayout();
	}

	/** Shrink a text node a step at a time until it fits `maxH`. */
	function fitText(node: KonvaText, text: string, sizes: number[], maxH: number) {
		for (const size of sizes) {
			node.fontSize(size);
			node.text(text);
			if (node.height() <= maxH) return;
		}
	}

	function update(model: CardModel) {
		couple.text(coupleLine(model));

		const guest = model.guest.trim().slice(0, MAX_GUEST_LEN);
		fitText(guestText, guest ? `Dear ${guest},` : '', GUEST_SIZES, 26);

		const msg = model.message.trim().slice(0, MAX_MESSAGE_LEN);
		fitText(messageText, msg, MSG_SIZES, MSG_BAND_H);
		// Centre the message in its band so 2- and 4-line versions both sit well.
		messageText.y(MSG_TOP + Math.max(0, (MSG_BAND_H - messageText.height()) / 2));

		dateText.text(model.date.trim().toUpperCase());
		layer.batchDraw();
	}

	async function toBlob(): Promise<Blob> {
		// Ensure fonts are ready before rasterising.
		await document.fonts.ready;
		const blob = await stage.toBlob({
			pixelRatio: EXPORT_PIXEL_RATIO,
			mimeType: 'image/png'
		});
		return blob as Blob;
	}

	return {
		setPhoto,
		hasPhoto: () => photoNode !== null,
		setZoom,
		nudge,
		resetFraming,
		update,
		toBlob,
		destroy: () => stage.destroy()
	};
}
