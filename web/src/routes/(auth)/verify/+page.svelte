<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { ApiError, auth } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';

	let status = $state<'verifying' | 'ok' | 'error'>('verifying');
	let message = $state('');

	onMount(async () => {
		const token = page.url.searchParams.get('token');
		if (!token) {
			status = 'error';
			message = 'This verification link is missing its token.';
			return;
		}
		try {
			await auth.verify(token);
			status = 'ok';
		} catch (err) {
			status = 'error';
			message =
				err instanceof ApiError ? err.message : 'We could not verify this link. It may have expired.';
		}
	});
</script>

<main class="grid min-h-screen place-items-center p-4">
	<Card.Root class="w-full max-w-sm text-center">
		<Card.Header>
			{#if status === 'verifying'}
				<div class="text-2xl font-extrabold tracking-tight">Verifying…</div>
				<Card.Description>Hang tight while we confirm your email.</Card.Description>
			{:else if status === 'ok'}
				<div class="text-2xl font-extrabold tracking-tight">Email verified</div>
				<Card.Description>Your account is ready. You can sign in now.</Card.Description>
			{:else}
				<div class="text-2xl font-extrabold tracking-tight">Verification failed</div>
				<Card.Description>{message}</Card.Description>
			{/if}
		</Card.Header>
		{#if status !== 'verifying'}
			<Card.Footer class="flex-col items-stretch gap-3">
				<Button href="/login">Go to sign in</Button>
			</Card.Footer>
		{/if}
	</Card.Root>
</main>
