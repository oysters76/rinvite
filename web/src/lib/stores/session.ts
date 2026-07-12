// Auth session: holds the JWT and persists it to localStorage. Kept as a plain
// module (no Svelte runes) so the API layer is trivially testable in Node; the
// UI layer can subscribe for reactivity or wrap it later.

const STORAGE_KEY = 'rinvite_token';
const hasStorage = typeof localStorage !== 'undefined';

let token: string | null = hasStorage ? localStorage.getItem(STORAGE_KEY) : null;
const listeners = new Set<() => void>();

function notify() {
	for (const fn of listeners) fn();
}

export function getToken(): string | null {
	return token;
}

export function setToken(value: string): void {
	token = value;
	if (hasStorage) localStorage.setItem(STORAGE_KEY, value);
	notify();
}

export function clearToken(): void {
	token = null;
	if (hasStorage) localStorage.removeItem(STORAGE_KEY);
	notify();
}

export function isAuthenticated(): boolean {
	return token !== null;
}

/** Subscribe to token changes (login/logout). Returns an unsubscribe fn. */
export function subscribe(fn: () => void): () => void {
	listeners.add(fn);
	return () => listeners.delete(fn);
}
