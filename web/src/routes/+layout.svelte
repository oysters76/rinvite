<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { goto } from '$app/navigation';
	import { onUnauthorized } from '$lib/api';
	import { Toaster } from '$lib/components/ui/sonner';

	// Any 401 from the API layer sends the user back to sign in.
	onUnauthorized(() => goto('/login'));

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<Toaster position="bottom-right" />
{@render children()}
