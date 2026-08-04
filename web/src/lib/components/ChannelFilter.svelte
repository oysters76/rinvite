<script lang="ts">
	import type { InviteChannel } from '$lib/api';

	type Value = InviteChannel | 'all';

	let {
		value,
		counts,
		onSelect
	}: {
		value: Value;
		counts: { all: number; einvite: number; print: number };
		onSelect: (v: Value) => void;
	} = $props();

	const options = $derived<{ value: Value; label: string; count: number }[]>([
		{ value: 'all', label: 'All', count: counts.all },
		{ value: 'einvite', label: 'E-invite', count: counts.einvite },
		{ value: 'print', label: 'Print', count: counts.print }
	]);
</script>

<div class="inline-flex border" role="group" aria-label="Filter by invite channel">
	{#each options as opt, i (opt.value)}
		{@const active = value === opt.value}
		<button
			type="button"
			aria-pressed={active}
			class="px-3 py-1.5 text-[13px] {i > 0 ? 'border-l' : ''} {active
				? 'bg-primary text-primary-foreground'
				: 'hover:bg-accent/50'}"
			onclick={() => onSelect(opt.value)}
		>
			{opt.label}
			<span class="ml-1.5 tabular-nums {active ? 'opacity-70' : 'text-muted-foreground'}"
				>{opt.count}</span
			>
		</button>
	{/each}
</div>
