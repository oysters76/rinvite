# rinvite — web

The organizer dashboard frontend for the [rinvite](../README.md) backend.

- **SvelteKit 2 + Svelte 5 (runes) + TypeScript**, built as a static SPA
  (`@sveltejs/adapter-static`, SSR off). It calls the backend API cross-origin.
- **Tailwind v4** and **shadcn-svelte** for the UI. The server adds the
  component primitives during UI work.

> **Status:** the foundation and a typed API layer only. There are no pages or
> screens yet. Those come in the next pass.

## Setup

```bash
cd web
npm install
cp .env.example .env      # point VITE_API_BASE_URL at your backend
```

## Develop

Run the backend (`JWT_SECRET=$(openssl rand -hex 32) cargo run` in the repo root),
then:

```bash
npm run dev       # http://localhost:5173
```

CORS is permissive when the backend's `CORS_ALLOWED_ORIGINS` is unset. For
parity, set it to `http://localhost:5173`.

## Scripts

```bash
npm run check     # svelte-check (types)
npm run test      # vitest (API-layer unit tests)
npm run build     # static build → build/
npm run lint      # eslint + prettier
```

## The API layer

`src/lib/api/` is a typed client for the backend:

- `client.ts` — the fetch core: the base URL (`VITE_API_BASE_URL`), bearer auth, JSON,
  `ApiError`, `204`, blob downloads, and an `onUnauthorized` hook.
- `types.ts` — the TypeScript mirrors of the backend DTOs.
- `auth.ts` · `events.ts` · `guests.ts` · `invites.ts` — the typed endpoint wrappers.
- `src/lib/stores/session.ts` — the JWT token holder. It persists to `localStorage`.

Usage:

```ts
import { auth, events, guests, invites, ApiError } from '$lib/api';

await auth.login(email, password);      // stores the token
const list = await events.list();
const guest = await guests.create(eventId, { name, channel: 'einvite', max_party_size: 2 });
const report = await invites.sendBatch(eventId);
```

## Deploy

Make a static build (`npm run build`), then deploy `build/` to any static host.
Set `VITE_API_BASE_URL` (at build time) to the production API URL, and set the
backend's `CORS_ALLOWED_ORIGINS` to this app's origin.
