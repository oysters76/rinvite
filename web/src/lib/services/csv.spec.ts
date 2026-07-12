import { describe, expect, it } from 'vitest';
import { parseGuestCsv } from './csv';

describe('parseGuestCsv', () => {
	it('parses rows, normalizes channel, and coerces party size', () => {
		const rows = parseGuestCsv(
			[
				'Mr Dhammika & family, einvite, d@example.com, , 2',
				'Aunty Kamala, print, , 0771234567, 1',
				'Ravi, PRINT, r@x.com, , 3'
			].join('\n')
		);
		expect(rows).toEqual([
			{ name: 'Mr Dhammika & family', channel: 'einvite', email: 'd@example.com', phone: null, max_party_size: 2 },
			{ name: 'Aunty Kamala', channel: 'print', email: null, phone: '0771234567', max_party_size: 1 },
			{ name: 'Ravi', channel: 'print', email: 'r@x.com', phone: null, max_party_size: 3 }
		]);
	});

	it('defaults unknown/blank channel to einvite and party size to 1', () => {
		const [row] = parseGuestCsv('Solo Guest');
		expect(row).toMatchObject({ name: 'Solo Guest', channel: 'einvite', max_party_size: 1 });
	});

	it('drops blank lines and rows without a name', () => {
		expect(parseGuestCsv('\n  \n, print, a@b.com')).toHaveLength(0);
	});
});
