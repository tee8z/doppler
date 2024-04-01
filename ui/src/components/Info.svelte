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
			<dt>Blockheight</dt>
			<dd>{info.block_height}</dd>
			<dt># Peers</dt>
			<dd>{info.num_peers}</dd>
			<dt># Active Channels</dt>
			<dd>{info.num_active_channels}</dd>
			<dt>Synced to Chain</dt>
			<dd>{info.synced_to_chain ? 'True' : 'False'}</dd>
			<dt>Synced to Graph</dt>
			<dd>{info.synced_to_graph ? 'True' : 'False'}</dd>
			<dt>Version</dt>
			<dd>{info.version}</dd>
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
