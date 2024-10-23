<script lang="ts">
	export let info: any = {};
</script>

{#if info == null || Object.keys(info).length == 0}
	<p>Loading...</p>
{:else}
	<div>
		<dl>
			<dt>Alias</dt>
			<dd>{info.alias}</dd>
			<dt>Version</dt>
			<dd>{info.version}</dd>
			<dt>Balance</dt>
			<dd><pre>{info.balance}</pre></dd>
		</dl>
	</div>
{/if}

<style>
	dl > dt {
		float: left;
		clear: both;
		margin-right: 10px; /* Adjust as needed */
	}
	dl > dd {
		margin-left: 0;
		overflow: auto; /* Clear the float */
	}
</style>
