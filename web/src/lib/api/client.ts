// The HTTP core: base URL, bearer auth, JSON encoding, typed errors, 204, and
// blob downloads. Every endpoint module builds on this.

import { clearToken, getToken } from '../stores/session';

export const apiBaseUrl = (
	(import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:3000'
).replace(/\/$/, '');

/** A failed API call. `status` is the HTTP status; `message` is server-supplied when available. */
export class ApiError extends Error {
	readonly status: number;
	constructor(status: number, message: string) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

// The UI registers a handler (e.g. redirect to /login) invoked on a 401. Kept
// as a hook so the API layer doesn't depend on the router.
let unauthorizedHandler: (() => void) | null = null;
export function onUnauthorized(handler: () => void): void {
	unauthorizedHandler = handler;
}

type Method = 'GET' | 'POST' | 'PATCH' | 'DELETE';

interface RequestOptions {
	method?: Method;
	body?: unknown;
	/** Attach the bearer token (default true). */
	auth?: boolean;
}

function authHeader(auth: boolean): Record<string, string> {
	if (!auth) return {};
	const token = getToken();
	return token ? { authorization: `Bearer ${token}` } : {};
}

async function errorMessage(res: Response): Promise<string> {
	try {
		const data = await res.json();
		if (data && typeof data.error === 'string') return data.error;
	} catch {
		/* body wasn't JSON */
	}
	return `Request failed with status ${res.status}`;
}

function handleUnauthorized(): never {
	clearToken();
	unauthorizedHandler?.();
	throw new ApiError(401, 'Unauthorized');
}

/** Core request: returns parsed JSON (or `undefined` for 204). Throws `ApiError` on failure. */
export async function request<T>(path: string, opts: RequestOptions = {}): Promise<T> {
	const { method = 'GET', body, auth = true } = opts;
	const headers: Record<string, string> = { ...authHeader(auth) };
	if (body !== undefined) headers['content-type'] = 'application/json';

	const res = await fetch(`${apiBaseUrl}${path}`, {
		method,
		headers,
		body: body !== undefined ? JSON.stringify(body) : undefined
	});

	if (res.status === 401 && auth) handleUnauthorized();
	if (!res.ok) throw new ApiError(res.status, await errorMessage(res));
	if (res.status === 204) return undefined as T;
	return (await res.json()) as T;
}

export const get = <T>(path: string, auth = true) => request<T>(path, { method: 'GET', auth });
export const post = <T>(path: string, body?: unknown, auth = true) =>
	request<T>(path, { method: 'POST', body, auth });
export const patch = <T>(path: string, body?: unknown, auth = true) =>
	request<T>(path, { method: 'PATCH', body, auth });
export const del = (path: string, auth = true) => request<void>(path, { method: 'DELETE', auth });

/** Fetch a binary response (e.g. a PDF) with its download filename. */
export async function downloadBlob(path: string): Promise<{ blob: Blob; filename: string }> {
	const res = await fetch(`${apiBaseUrl}${path}`, { headers: authHeader(true) });
	if (res.status === 401) handleUnauthorized();
	if (!res.ok) throw new ApiError(res.status, await errorMessage(res));
	const blob = await res.blob();
	return { blob, filename: filenameFromDisposition(res.headers.get('content-disposition')) };
}

function filenameFromDisposition(header: string | null): string {
	if (header) {
		const match = /filename="?([^"]+)"?/.exec(header);
		if (match) return match[1];
	}
	return 'download';
}

/** Browser-only: trigger a file download for a blob. */
export function saveBlob(blob: Blob, filename: string): void {
	const url = URL.createObjectURL(blob);
	const a = document.createElement('a');
	a.href = url;
	a.download = filename;
	document.body.appendChild(a);
	a.click();
	a.remove();
	URL.revokeObjectURL(url);
}
