<script lang="ts">
	import * as AlertDialog from '$lib/components/ui/alert-dialog';

	let {
		open = $bindable(false),
		title,
		description = '',
		confirmLabel = 'Confirm',
		onconfirm
	}: {
		open?: boolean;
		title: string;
		description?: string;
		confirmLabel?: string;
		onconfirm: () => void;
	} = $props();

	function confirm() {
		onconfirm();
		open = false;
	}
</script>

<AlertDialog.Root bind:open>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{title}</AlertDialog.Title>
			{#if description}
				<AlertDialog.Description>{description}</AlertDialog.Description>
			{/if}
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
			<AlertDialog.Action onclick={confirm}>{confirmLabel}</AlertDialog.Action>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
