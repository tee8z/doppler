<script lang="ts">
	import { LndRequests, getConnections } from '$lib/lnd_requests';
	import type { Connections, Nodes } from '$lib/lnd_requests';
	import { onDestroy, onMount, tick } from 'svelte';
	import Graph from './Graph.svelte';
	import Info from './Info.svelte';
	import Button from './Button.svelte';
	let dataPromise: Promise<Nodes> | null = null;
	let poller: ReturnType<typeof setInterval>;
	let edges: any[] = [];
	let nodes: any[] = [];

	let info: any = null;
	let jsonData: any = null;
	let nodeConnections: { pubkey: string; connection: LndRequests }[] = [];
	function setNode(node: any) {
		console.log(node.info);
		info = node.info;
	}

	const fetchData = async (connections: Connections) => {
		let nodeData: Nodes = {};
		let nodesWeKnow: Record<string, string> = {};
		const promises = Object.keys(connections).map(async (key) => {
			const connectionConfig = connections[key];
			let requests = new LndRequests(
				connectionConfig.host,
				connectionConfig.macaroon,
				connectionConfig.tls
			);
			let graph = await requests.fetchGraph();
			let response = await requests.fetchChannels();
			let channels = response.channels;
			let balance = await requests.fetchBalance();
			let info = await requests.fetchInfo();
			nodeConnections.push({ pubkey: info.identity_pubkey, connection: requests });
			return { [key]: { graph, channels, balance, info } };
		});
		const results = await Promise.all(promises);
		results.forEach((result) => {
			const [key, data] = Object.entries(result)[0];
			nodeData[key] = data;
			nodesWeKnow[data.info.identity_pubkey] = key;
		});
		let key = Object.keys(nodeData)[0];
		//Set starting node
		setNode(nodeData[key]);

		Object.entries(nodeData).forEach(([key, value]) => {
			let current_pubkey = value.info.identity_pubkey;
			nodes.push({ id: current_pubkey, alias: key, nodeInfo: value.info, known: current_pubkey });
			value.channels.forEach((channel: any) => {
				let knownNode = nodesWeKnow[channel.remote_pubkey];
				if (!knownNode) {
					console.log(channel);
					if (channel.peer_alias) {
						nodes.push({
							id: channel.remote_pubkey,
							alias: channel.peer_alias,
							known: current_pubkey
						});
					} else {
						nodes.push({
							id: channel.remote_pubkey,
							alias: channel.remote_pubkey,
							known: current_pubkey
						});
					}
				}

				edges.push({
					source: current_pubkey,
					target: channel.remote_pubkey,
					channel_id: channel.chan_id,
					capacity: channel.capacity,
					local_balance: channel.local_balance,
					remote_balance: channel.remote_balance,
					initiator: channel.initiator,
					channel: channel
				});
			});
		});

		dataPromise = Promise.resolve(nodeData);
	};

	onMount(async () => {
		if (poller) {
			clearInterval(poller);
		}
		tick();
		let connections = await getConnections();
		fetchData(connections);
		//poller = setInterval(() => fetchData(connections), 20000); // Poll every 20 seconds
	});

	onDestroy(() => {
		if (poller) {
			clearInterval(poller);
		}
	});
	function handleClickData(event: any) {
		console.log('Received data from child:', event);
		const data = event.detail;
		if (data.type == 'channel') {
			console.log(data);
			jsonData = data.channel;
		} else if (data.type == 'node') {
			console.log(data);
			if (data.id != data.known) {
				let connection = nodeConnections.find((connection) => connection.pubkey == data.known);
				console.log(connection);
				connection?.connection.fetchSpecificNodeInfo(data.id).then((nodeInfo) => {
					jsonData = nodeInfo;
				});
			} else {
				jsonData = data.nodeInfo;
			}
		} else {
			console.log('data type not supported', data);
		}
	}
	function prettyPrintJson(jsonData: any) {
		return JSON.stringify(jsonData, null, 2);
	}
</script>

{#await dataPromise}
	<p>Loading graph...</p>
{:then nodeData}
	<div class="info">
		<div>
			<h1>Visualize</h1>
			<Info {info} />
			<div>
				{#if nodeData}
					{#each Object.keys(nodeData) as key}
						<Button on:click={() => setNode(nodeData[key])}>{key}</Button>
					{/each}
				{/if}
			</div>
			<div>
				{#if jsonData}
					<pre class="detail">{prettyPrintJson(jsonData)}</pre>
				{/if}
			</div>
		</div>
	</div>
	<div class="flex flex-1">
		{#if nodes.length > 0}
			<Graph on:dataEvent={handleClickData} {nodes} {edges} />
		{/if}
	</div>
{:catch error}
	<p>Error: {error.message}</p>
{/await}

<style>
	.info {
		flex: 0 0 30%; /* Adjust the width as needed */
		background: rgba(0, 151, 19, 0.1);
		margin: 15px;
		padding: 10px;
		font-size: large;
		overflow-wrap: break-word;
		white-space: pre-wrap;
		word-break: break-all;
		height: 100vh;
		overflow-y: auto; 
	}
	.detail {
		font-size: small;
		overflow-wrap: break-word;
		white-space: pre-wrap;
		word-break: break-all;
	}
</style>
