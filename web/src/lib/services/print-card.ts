// Sends a rendered card straight to the printer on 4x6 (4R) photo paper.
//
// The thank-you card already exports at 1200x1800 px = 4x6 in @ 300 DPI (see
// EXPORT_PIXEL_RATIO in thankyou-card.ts), so all that is missing is a page the
// browser will print at exactly that physical size.
//
// We print into a hidden same-origin iframe rather than into the app document.
// The card lives in a bits-ui dialog that is portalled to <body>, positioned
// `fixed` and transformed, over a body that gets `overflow: hidden` while the
// dialog is open — and Tailwind's preflight is global. A `@media print` rule
// that hides "everything except this one node" has to fight all of that. An
// iframe has its own document, so `@page { size: 4in 6in; margin: 0 }` is the
// only page rule in force.
//
// NOTE: browsers honour `@page size` for the page *box*, but the paper the user
// picks in the print dialog still wins. The UI tells them to choose 4x6 in /
// 10x15 cm at 100% scale.

/**
 * How the card sits on the sheet.
 *
 * - `borderless` — card fills the 4x6 page. Photo printers in borderless mode
 *   overscan by ~2-3% and crop the overflow; the card's ivory paper runs to the
 *   trim edge and the gold frame is inset 4%, so the frame survives.
 * - `margin` — the card is scaled down inside the sheet, leaving a white border.
 *   Safe on any printer, including ones with hardware margins.
 */
export type PrintFit = 'borderless' | 'margin';

/** Inset used by the `margin` fit, per edge. */
export const PRINT_MARGIN_MM = 3;

/** How long to wait for `afterprint` before cleaning up anyway. */
const CLEANUP_FALLBACK_MS = 60_000;

const escapeHtml = (s: string) =>
	s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/**
 * The whole print document, as a string. Pure, so the page geometry can be
 * unit-tested without a browser.
 */
export function printSheetHtml(opts: { title: string; src: string; fit: PrintFit }): string {
	const { title, src, fit } = opts;
	// `contain` keeps the 2:3 ratio while fitting inside the inset box rather than
	// stretching to it. Width binds, so the printed margin is 3mm left/right and
	// ~4.5mm top/bottom — both comfortably clear of any hardware margin.
	const image =
		fit === 'margin'
			? `max-width: calc(4in - ${PRINT_MARGIN_MM * 2}mm);
      max-height: calc(6in - ${PRINT_MARGIN_MM * 2}mm);
      object-fit: contain;`
			: `width: 4in;
      height: 6in;`;
	return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>${escapeHtml(title)}</title>
<style>
  @page { size: 4in 6in; margin: 0; }
  html, body {
    margin: 0;
    padding: 0;
    width: 4in;
    height: 6in;
    background: #fff;
    /* Without this the ivory paper and gilt linework get washed out by the
       browser's default "don't print backgrounds" behaviour. */
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
  body { display: flex; align-items: center; justify-content: center; }
  img { display: block; ${image} }
</style>
</head>
<body><img src="${escapeHtml(src)}" alt="${escapeHtml(title)}"></body>
</html>`;
}

function loadImage(doc: Document): Promise<void> {
	const img = doc.querySelector('img');
	if (!img) return Promise.reject(new Error('print sheet has no image'));
	if (img.complete && img.naturalWidth > 0) return Promise.resolve();
	return new Promise((resolve, reject) => {
		img.addEventListener('load', () => resolve(), { once: true });
		img.addEventListener('error', () => reject(new Error('print image failed to load')), {
			once: true
		});
	});
}

/**
 * Open the print dialog for `blob` on a 4x6 sheet.
 *
 * Resolves once the dialog has been handed off — `window.print()` blocks until
 * the user dismisses it in most browsers, but not all, so callers should not
 * treat resolution as "the card was printed". Rejects if the sheet could not be
 * built at all, in which case the caller should fall back to the PNG download.
 */
export async function printImage(
	blob: Blob,
	opts: { title: string; fit: PrintFit }
): Promise<void> {
	const url = URL.createObjectURL(blob);
	const frame = document.createElement('iframe');
	// Off-screen rather than `display: none` — a hidden iframe has no layout and
	// Chrome refuses to print it.
	frame.setAttribute('aria-hidden', 'true');
	frame.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;';

	let cleanedUp = false;
	let timer: ReturnType<typeof setTimeout> | undefined;
	const cleanup = () => {
		if (cleanedUp) return;
		cleanedUp = true;
		clearTimeout(timer);
		URL.revokeObjectURL(url);
		frame.remove();
	};

	try {
		document.body.appendChild(frame);
		const win = frame.contentWindow;
		const doc = frame.contentDocument;
		if (!win || !doc) throw new Error('could not open a print frame');

		doc.open();
		doc.write(printSheetHtml({ title: opts.title, src: url, fit: opts.fit }));
		doc.close();
		await loadImage(doc);

		// The blob URL and the frame have to outlive print(), which returns
		// immediately in browsers that render the dialog asynchronously.
		win.addEventListener('afterprint', cleanup, { once: true });
		timer = setTimeout(cleanup, CLEANUP_FALLBACK_MS);

		win.focus();
		win.print();
	} catch (e) {
		cleanup();
		throw e;
	}
}
