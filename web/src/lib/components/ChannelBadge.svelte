<script lang="ts">
	import type { InviteChannel } from '$lib/api';
	import Tag from './Tag.svelte';

	let { channel }: { channel: InviteChannel } = $props();
</script>

{#if channel === 'einvite'}
	<Tag variant="accent">E-invite</Tag>
{:else}
	<Tag variant="outline">Print</Tag>
{/if}
