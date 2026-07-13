<script lang="ts">
	import { ApiError, auth } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import * as Card from '$lib/components/ui/card';
	import { toast } from 'svelte-sonner';

	let email = $state('');
	let password = $state('');
	let loading = $state(false);
	let submitted = $state(false);
	let resending = $state(false);

	async function submit(e: SubmitEvent) {
		e.preventDefault();
		if (password.length < 8) {
			toast.error('Password must be at least 8 characters.');
			return;
		}
		loading = true;
		try {
			await auth.signup(email, password);
			submitted = true;
		} catch (err) {
			toast.error(err instanceof ApiError ? err.message : 'Could not create account');
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
	{#if submitted}
		<Card.Root class="w-full max-w-sm">
			<Card.Header>
				<div class="text-2xl font-extrabold tracking-tight">Check your email</div>
				<Card.Description>
					We sent a verification link to <strong>{email}</strong>. Click it to activate your
					account, then sign in.
				</Card.Description>
			</Card.Header>
			<Card.Footer class="flex-col items-stretch gap-3">
				<Button variant="secondary" onclick={resend} disabled={resending}>
					{resending ? 'Sending…' : 'Resend verification email'}
				</Button>
				<p class="text-muted-foreground text-center text-sm">
					<a href="/login" class="text-primary">Back to sign in</a>
				</p>
			</Card.Footer>
		</Card.Root>
	{:else}
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
	{/if}
</main>
