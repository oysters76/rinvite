import { describe, expect, it } from 'vitest';
import { parseGuestCsvFile } from './csv';

describe('parseGuestCsvFile', () => {
	it('maps columns by header regardless of order', () => {
		const rows = parseGuestCsvFile(
			[
				'channel,name,max_party_size,phone,email',
				'einvite,Ann,2,,a@x.com',
				'print,Bo,1,0771234567,'
			].join('\n')
		);
		expect(rows).toEqual([
			{ name: 'Ann', channel: 'einvite', email: 'a@x.com', phone: null, max_party_size: 2 },
			{ name: 'Bo', channel: 'print', email: null, phone: '0771234567', max_party_size: 1 }
		]);
	});

	it('tolerates missing optional columns and a header-only file', () => {
		const rows = parseGuestCsvFile(['name', 'Solo Guest'].join('\n'));
		expect(rows).toEqual([
			{ name: 'Solo Guest', channel: 'einvite', email: null, phone: null, max_party_size: 1 }
		]);
		expect(parseGuestCsvFile('name,channel')).toHaveLength(0);
	});

	it('accepts "max party size" alias and normalizes channel case', () => {
		const [row] = parseGuestCsvFile(['name,channel,max party size', 'Ravi,PRINT,3'].join('\n'));
		expect(row).toMatchObject({ name: 'Ravi', channel: 'print', max_party_size: 3 });
	});

	it('handles quoted fields with commas and CRLF endings', () => {
		const rows = parseGuestCsvFile('name,email\r\n"Dhammika, & family",d@example.com\r\n');
		expect(rows).toEqual([
			{
				name: 'Dhammika, & family',
				channel: 'einvite',
				email: 'd@example.com',
				phone: null,
				max_party_size: 1
			}
		]);
	});

	it('returns nothing when there is no name column', () => {
		expect(parseGuestCsvFile(['email,channel', 'a@b.com,print'].join('\n'))).toHaveLength(0);
	});
});
