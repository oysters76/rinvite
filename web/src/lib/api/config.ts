import { get } from './client';
import type { AppConfig } from './types';

let cached: AppConfig | null = null;

export const config = {
	/** Fetch public client config (cached for the session). */
	async get(): Promise<AppConfig> {
		if (!cached) cached = await get<AppConfig>('/config', false);
		return cached;
	}
};
