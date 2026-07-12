import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearToken, getToken, setToken } from '../stores/session';
import { ApiError, apiBaseUrl, downloadBlob, get, onUnauthorized, post, request } from './client';
import { events } from './events';

type FetchImpl = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

function mockFetch(handler: FetchImpl) {
	return vi.spyOn(globalThis, 'fetch').mockImplementation(handler as typeof fetch);
}

function jsonRes(body: unknown, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});
}

function lastInit(spy: ReturnType<typeof mockFetch>): RequestInit {
	return spy.mock.calls.at(-1)![1] as RequestInit;
}

beforeEach(() => {
	clearToken();
	onUnauthorized(() => {});
});
afterEach(() => vi.restoreAllMocks());

describe('request core', () => {
	it('prefixes the base URL and returns parsed JSON', async () => {
		const f = mockFetch(async () => jsonRes({ ok: 1 }));
		const out = await get<{ ok: number }>('/events');
		expect(out).toEqual({ ok: 1 });
		expect(f).toHaveBeenCalledWith(
			`${apiBaseUrl}/events`,
			expect.objectContaining({ method: 'GET' })
		);
	});

	it('attaches the bearer token when authenticated', async () => {
		setToken('abc');
		const f = mockFetch(async () => jsonRes({}));
		await get('/events');
		expect((lastInit(f).headers as Record<string, string>).authorization).toBe('Bearer abc');
	});

	it('encodes a JSON body and omits auth when auth=false', async () => {
		setToken('abc');
		const f = mockFetch(async () => jsonRes({ token: 't' }));
		await post('/auth/login', { email: 'a@b.com' }, false);
		const init = lastInit(f);
		const headers = init.headers as Record<string, string>;
		expect(headers.authorization).toBeUndefined();
		expect(headers['content-type']).toBe('application/json');
		expect(init.body).toBe(JSON.stringify({ email: 'a@b.com' }));
	});

	it('maps a non-2xx {error} body to ApiError', async () => {
		mockFetch(async () => jsonRes({ error: 'end time must be after start time' }, 400));
		await expect(get('/events')).rejects.toMatchObject({
			name: 'ApiError',
			status: 400,
			message: 'end time must be after start time'
		});
	});

	it('resolves to undefined on 204', async () => {
		mockFetch(async () => new Response(null, { status: 204 }));
		await expect(request('/events/1', { method: 'DELETE' })).resolves.toBeUndefined();
	});

	it('on 401 clears the token and invokes the unauthorized handler', async () => {
		setToken('abc');
		const handler = vi.fn();
		onUnauthorized(handler);
		mockFetch(async () => new Response(null, { status: 401 }));
		await expect(get('/events')).rejects.toBeInstanceOf(ApiError);
		expect(getToken()).toBeNull();
		expect(handler).toHaveBeenCalledOnce();
	});
});

describe('endpoint modules', () => {
	it('events.create POSTs to /events with the payload', async () => {
		const f = mockFetch(async () => jsonRes({ id: 'e1' }));
		await events.create({ bride_name: 'Hansika' } as never);
		expect(f).toHaveBeenCalledWith(
			`${apiBaseUrl}/events`,
			expect.objectContaining({ method: 'POST' })
		);
		expect(lastInit(f).body).toContain('Hansika');
	});
});

describe('downloadBlob', () => {
	it('returns the blob and parses the Content-Disposition filename', async () => {
		mockFetch(
			async () =>
				new Response(new Blob(['%PDF-1.7']), {
					status: 200,
					headers: { 'content-disposition': 'attachment; filename="invitations.pdf"' }
				})
		);
		const { blob, filename } = await downloadBlob('/events/e1/invites/print.pdf');
		expect(filename).toBe('invitations.pdf');
		expect(await blob.text()).toContain('%PDF');
	});
});
