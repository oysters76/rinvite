<script lang="ts">
	import AppNav from '$lib/components/AppNav.svelte';

	let { children } = $props();
</script>

<div class="flex min-h-screen flex-col">
	<AppNav />
	{@render children()}
</div>
