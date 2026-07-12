<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores/auth.svelte';
	import { buttonVariants } from '$lib/components/ui/button';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { LogOut } from '@lucide/svelte';

	let { crumb }: { crumb?: string } = $props();

	function logout() {
		authStore.logout();
		goto('/login');
	}
</script>

<nav class="flex items-center gap-4 border-b-2 px-4 py-3">
	<a href="/events" class="text-lg font-extrabold tracking-tight">Rinvite</a>
	<a href="/events" class="text-sm hover:text-primary">Events</a>
	{#if crumb}
		<span class="text-muted-foreground truncate text-[13px]">{crumb}</span>
	{/if}
	<span class="flex-1"></span>
	<DropdownMenu.Root>
		<DropdownMenu.Trigger class={buttonVariants({ variant: 'ghost', size: 'sm' })}>
			{authStore.user?.email ?? 'Account'}
		</DropdownMenu.Trigger>
		<DropdownMenu.Content align="end">
			<DropdownMenu.Item onclick={logout}>
				<LogOut class="size-4" />
				Sign out
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
</nav>
