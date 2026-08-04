import { describe, expect, it } from 'vitest';
import { PRINT_MARGIN_MM, printSheetHtml } from './print-card';

const sheet = (fit: 'borderless' | 'margin', title = 'thank-you-Amaya') =>
	printSheetHtml({ title, src: 'blob:http://localhost/abc', fit });

describe('printSheetHtml', () => {
	it('always asks for a 4x6 page with no page margins', () => {
		for (const fit of ['borderless', 'margin'] as const) {
			const html = sheet(fit);
			expect(html).toContain('@page { size: 4in 6in; margin: 0; }');
			// Otherwise the browser drops the ivory paper and the gilt linework.
			expect(html).toContain('print-color-adjust: exact');
		}
	});

	it('fills the sheet edge to edge when borderless', () => {
		const html = sheet('borderless');
		expect(html).toContain('width: 4in;');
		expect(html).toContain('height: 6in;');
		expect(html).not.toContain('object-fit: contain');
	});

	it('insets the card and keeps its ratio when a margin is wanted', () => {
		const html = sheet('margin');
		expect(html).toContain(`max-width: calc(4in - ${PRINT_MARGIN_MM * 2}mm)`);
		expect(html).toContain(`max-height: calc(6in - ${PRINT_MARGIN_MM * 2}mm)`);
		// `contain`, not `fill` — a 2:3 card in a non-2:3 box must not stretch.
		expect(html).toContain('object-fit: contain');
	});

	it('escapes the title, which carries a guest-typed name', () => {
		// The title becomes the suggested filename when the user prints to PDF, so
		// it is attacker-influenced markup sitting in a document we generate.
		const html = sheet('margin', 'thank-you-<script>alert("x")</script> & co');
		expect(html).toContain(
			'<title>thank-you-&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; co</title>'
		);
		expect(html).not.toContain('<script>');
	});
});
