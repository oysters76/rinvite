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

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		loading = true;
		try {
			await auth.login(email, password);
			await goto('/events');
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not sign in');
		} finally {
			loading = false;
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
