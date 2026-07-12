<script lang="ts">
	import type { GuestStats } from '$lib/services';

	let { stats }: { stats: GuestStats } = $props();

	const items = $derived([
		{ label: 'Total guests', value: stats.total, accent: false },
		{ label: 'E-invite', value: stats.einvite, accent: false },
		{ label: 'Print', value: stats.print, accent: false },
		{ label: 'Attending', value: stats.attending, accent: true },
		{ label: 'Pending', value: stats.pending, accent: false },
		{ label: 'Declined', value: stats.declined, accent: false }
	]);
</script>

<div class="flex flex-wrap gap-6">
	{#each items as item, i (item.label)}
		<div class="pr-6" class:border-r-2={i < items.length - 1}>
			<div class="text-2xl font-semibold" class:text-primary={item.accent}>{item.value}</div>
			<div class="kicker">{item.label}</div>
		</div>
	{/each}
</div>
