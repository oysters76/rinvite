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
		if (password.length < 8) {
			toast.error('Password must be at least 8 characters.');
			return;
		}
		loading = true;
		try {
			await auth.signup(email, password);
			await goto('/events');
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not create account');
		} finally {
			loading = false;
		}
	}
</script>

<main class="grid min-h-screen place-items-center p-4">
	<Card.Root class="w-full max-w-sm">
		<Card.Header>
			<div class="text-2xl font-extrabold tracking-tight">Create your account</div>
			<Card.Description>Start sending beautiful invitations.</Card.Description>
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
						autocomplete="new-password"
					/>
				</div>
			</Card.Content>
			<Card.Footer class="mt-4 flex-col items-stretch gap-3">
				<Button type="submit" disabled={loading}>{loading ? 'Creating…' : 'Create account'}</Button>
				<p class="text-muted-foreground text-center text-sm">
					Already have an account? <a href="/login" class="text-primary">Sign in</a>
				</p>
			</Card.Footer>
		</form>
	</Card.Root>
</main>
