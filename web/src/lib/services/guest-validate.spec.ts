import { describe, expect, it } from 'vitest';
import type { CreateGuest } from '$lib/api';
import { validateGuestRow } from './guest-validate';

const base: CreateGuest = {
	name: 'Ann',
	channel: 'einvite',
	email: null,
	phone: null,
	max_party_size: 2
};

describe('validateGuestRow', () => {
	it('passes a well-formed row', () => {
		expect(validateGuestRow(base)).toEqual([]);
		expect(validateGuestRow({ ...base, email: 'a@b.com', phone: '0771234567' })).toEqual([]);
	});

	it('flags an empty name', () => {
		expect(validateGuestRow({ ...base, name: '   ' })).toContain('name must not be empty');
	});

	it('flags a party size below one or non-integer', () => {
		expect(validateGuestRow({ ...base, max_party_size: 0 })).toHaveLength(1);
		expect(validateGuestRow({ ...base, max_party_size: 1.5 })).toHaveLength(1);
	});

	it('flags a malformed email', () => {
		expect(validateGuestRow({ ...base, email: 'no-at-sign' })).toHaveLength(1);
		expect(validateGuestRow({ ...base, email: 'a@b' })).toHaveLength(1);
	});

	it('flags an over-long phone', () => {
		expect(validateGuestRow({ ...base, phone: '1'.repeat(33) })).toContain(
			'phone must be 1–32 characters'
		);
	});
});
