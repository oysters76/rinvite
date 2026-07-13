<script lang="ts">
	import { goto } from '$app/navigation';
	import { ApiError, auth } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import * as Card from '$lib/components/ui/card';
	import { toast } from 'svelte-sonner';

	let email = $state('');
	let password = $state('');
	let loading = $state(false);
	let needsVerification = $state(false);
	let resending = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		loading = true;
		try {
			await auth.login(email, password);
			await goto('/events');
		} catch (err) {
			// 403 = correct credentials but the email isn't verified yet.
			if (err instanceof ApiError && err.status === 403) {
				needsVerification = true;
			} else {
				toast.error(err instanceof ApiError ? err.message : 'Could not sign in');
			}
		} finally {
			loading = false;
		}
	}

	async function resend() {
		resending = true;
		try {
			await auth.resendVerification(email);
			toast.success('Verification email sent.');
		} catch {
			toast.error('Could not resend the email.');
		} finally {
			resending = false;
		}
	}
</script>

<main class="grid min-h-screen place-items-center p-4">
	<Card.Root class="w-full max-w-sm">
		<Card.Header>
			<div class="text-2xl font-extrabold tracking-tight">Rinvite</div>
			<Card.Description>Sign in to manage your events.</Card.Description>
		</Card.Header>
		<form onsubmit={submit}>
			<Card.Content class="grid gap-4">
				{#if needsVerification}
					<div class="rounded-md border border-amber-300 bg-amber-50 p-3 text-sm text-amber-900">
						<p>Please verify your email before signing in. Check your inbox for the link.</p>
						<button
							type="button"
							class="text-primary mt-1 underline disabled:opacity-50"
							onclick={resend}
							disabled={resending}
						>
							{resending ? 'Sending…' : 'Resend verification email'}
						</button>
					</div>
				{/if}
				<div class="grid gap-1.5">
					<Label for="email">Email</Label>
					<Input id="email" type="email" bind:value={email} required autocomplete="email" />
				</div>
				<div class="grid gap-1.5">
					<Label for="password">Password</Label>
					<Input
						id="password"
						type="password"
						bind:value={password}
						required
						autocomplete="current-password"
					/>
				</div>
			</Card.Content>
			<Card.Footer class="mt-4 flex-col items-stretch gap-3">
				<Button type="submit" disabled={loading}>{loading ? 'Signing in…' : 'Sign in'}</Button>
				<p class="text-muted-foreground text-center text-sm">
					No account? <a href="/signup" class="text-primary">Create one</a>
				</p>
			</Card.Footer>
		</form>
	</Card.Root>
</main>
