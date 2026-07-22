import type { CreateGuest, InviteChannel } from '$lib/api';

/**
 * Parse the text of an uploaded `.csv` file into guest rows. The first
 * non-empty line is treated as a header and columns are mapped by name
 * (case-insensitive), so column order is flexible. Only `name` is required;
 * `channel`, `email`, `phone`, and `max_party_size` are optional. Quoted
 * fields (with embedded commas, quotes, or newlines) are supported.
 *
 * Pure and dependency-free so it's trivially testable.
 */
export function parseGuestCsvFile(text: string): CreateGuest[] {
	const records = parseCsv(text).filter((r) => r.some((cell) => cell.trim().length > 0));
	if (records.length < 2) return []; // need a header + at least one data row

	const header = records[0].map((h) => normalizeHeader(h));
	const col = (...aliases: string[]) => {
		for (const a of aliases) {
			const i = header.indexOf(a);
			if (i !== -1) return i;
		}
		return -1;
	};
	const nameIdx = col('name');
	if (nameIdx === -1) return []; // no name column → nothing we can import
	const channelIdx = col('channel');
	const emailIdx = col('email');
	const phoneIdx = col('phone');
	const maxIdx = col('max_party_size', 'max party size', 'max', 'party size', 'party_size');

	return records
		.slice(1)
		.map((cells) => {
			const at = (i: number) => (i === -1 ? '' : (cells[i] ?? '')).trim();
			return toGuest([at(nameIdx), at(channelIdx), at(emailIdx), at(phoneIdx), at(maxIdx)]);
		})
		.filter((row) => row.name.length > 0);
}

/** Build a `CreateGuest` from raw `[name, channel, email, phone, max]` strings. */
function toGuest([name, channelRaw, email, phone, maxRaw]: (string | undefined)[]): CreateGuest {
	const channel: InviteChannel = (channelRaw ?? '').toLowerCase() === 'print' ? 'print' : 'einvite';
	const max = Number(maxRaw);
	return {
		name: (name ?? '').trim(),
		channel,
		email: email?.trim() || null,
		phone: phone?.trim() || null,
		max_party_size: Number.isFinite(max) && max >= 1 ? Math.floor(max) : 1
	} satisfies CreateGuest;
}

function normalizeHeader(h: string): string {
	return h.trim().toLowerCase().replace(/^"|"$/g, '');
}

/**
 * A minimal RFC-4180-ish CSV tokenizer: comma-separated fields, `"`-quoted
 * fields that may contain commas, newlines, and escaped quotes (`""`), and
 * either `\n` or `\r\n` line endings. Returns an array of records (rows), each
 * an array of raw cell strings.
 */
function parseCsv(text: string): string[][] {
	const rows: string[][] = [];
	let row: string[] = [];
	let field = '';
	let inQuotes = false;

	for (let i = 0; i < text.length; i++) {
		const c = text[i];
		if (inQuotes) {
			if (c === '"') {
				if (text[i + 1] === '"') {
					field += '"';
					i++; // skip the escaped quote
				} else {
					inQuotes = false;
				}
			} else {
				field += c;
			}
		} else if (c === '"') {
			inQuotes = true;
		} else if (c === ',') {
			row.push(field);
			field = '';
		} else if (c === '\n' || c === '\r') {
			// Close the field/row on a line break; swallow the \n of a \r\n pair.
			if (c === '\r' && text[i + 1] === '\n') i++;
			row.push(field);
			rows.push(row);
			row = [];
			field = '';
		} else {
			field += c;
		}
	}
	// Flush a trailing field/row if the text didn't end with a newline.
	if (field.length > 0 || row.length > 0) {
		row.push(field);
		rows.push(row);
	}
	return rows;
}
