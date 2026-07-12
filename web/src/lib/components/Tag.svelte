<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		variant = 'neutral',
		children
	}: { variant?: 'accent' | 'outline' | 'neutral'; children: Snippet } = $props();

	const styles: Record<string, string> = {
		accent: 'background:var(--accent-100);color:var(--accent-800)',
		neutral: 'background:var(--neutral-100);color:var(--neutral-800)',
		outline: 'border:1px solid var(--primary);color:var(--primary)'
	};
</script>

<span
	class="inline-flex items-center px-2.5 py-[3px] text-[11px] tracking-wide"
	style={styles[variant]}
>
	{@render children()}
</span>
