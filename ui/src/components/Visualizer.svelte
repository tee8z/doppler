<script lang="ts">
	import { LndRequests } from '$lib/lnd_requests';
	import { onDestroy, onMount, tick } from 'svelte';
	import Graph from './Graph.svelte';
	import Info from './Info.svelte';
	import Button from './Button.svelte';
	import type { Connections } from '../routes/api/connections/+server';
	import { getConnections } from '$lib/connections';
	import type { NodeRequests, Nodes } from '$lib/nodes';
	import { CorelnRequests } from '$lib/coreln_requests';
	import { EclairRequests } from '$lib/eclair_requests';
	let dataPromise: Promise<Nodes> | null = null;
	let poller: ReturnType<typeof setInterval>;
	let edges: any[] = [];
	let nodes: any[] = [];

	let info: any = null;
	let jsonData: any = null;
	let nodeConnections: { pubkey: string; connection: NodeRequests }[] = [];
	function setNode(node: any) {
		info = node.info;
		jsonData = node.info;
	}

	const fetchData = async (connections: Connections) => {
		let nodeData: Nodes = {};
		let nodesWeKnow: Record<string, string> = {};
		const promises = Object.keys(connections).map(async (key) => {
			const connectionConfig = connections[key];
			if (connectionConfig.type === 'lnd') {
				let requests = new LndRequests(
					connectionConfig.host,
					connectionConfig.macaroon,
					connectionConfig.tls
				);
				let response = await requests.fetchChannels();
				let channels = response.channels;
				let balance = await requests.fetchBalance();
				let info = await requests.fetchInfo();
				nodesWeKnow[info.identity_pubkey] = key;
				nodeConnections.push({ pubkey: info.identity_pubkey, connection: requests });
				return { [key]: { channels, balance, info, type: 'lnd' } };
			} else if (connectionConfig.type === 'coreln') {
				const requests = new CorelnRequests(connectionConfig.host, connectionConfig.macaroon);
				const channels = await requests.fetchChannels();
				const balance = await requests.fetchBalance();
				const info = await requests.fetchInfo();
				nodesWeKnow[info.id] = key;
				nodeConnections.push({ pubkey: info.id, connection: requests });
				return { [key]: { channels, balance, info, type: 'coreln' } };
			} else if (connectionConfig.type === 'eclair') {
				const requests = new EclairRequests(connectionConfig.host, connectionConfig.password);
				const channels = await requests.fetchChannels();
				const balance = await requests.fetchBalance();
				const info = await requests.fetchInfo();
				nodesWeKnow[info.nodeId] = key;
				nodeConnections.push({ pubkey: info.nodeId, connection: requests });
				return { [key]: { channels, balance, info, type: 'eclair' } };
			}
		});
		const results = await Promise.all(promises);
		results.forEach((result) => {
			if (!result) {
				console.error('issue building the requests client');
				return;
			}
			const [key, data] = Object.entries(result)[0];
			nodeData[key] = data;
		});
		let key = Object.keys(nodeData)[0];
		//Set starting node
		setNode(nodeData[key]);
		edges = [];
		nodes = [];
		map_lnd_channels(nodes, nodesWeKnow, edges, nodeData);
		map_coreln_channels(nodes, nodesWeKnow, edges, nodeData);
		map_eclair_channels(nodes, nodesWeKnow, edges, nodeData);

		dataPromise = Promise.resolve(nodeData);
	};

	function map_lnd_channels(
		nodes: any[],
		nodesWeKnow: Record<string, string>,
		edges: any[],
		nodeData: Nodes
	) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'lnd') {
				return;
			}
			let current_pubkey = value.info.identity_pubkey;
			nodes.push({ id: current_pubkey, alias: key, nodeInfo: value.info, known: current_pubkey });
			value.channels.forEach((channel: any) => {
				let knownNode = nodesWeKnow[channel.remote_pubkey];
				if (!knownNode) {
					if (channel.peer_alias) {
						nodes.push({
							id: channel.remote_pubkey,
							alias: channel.peer_alias,
							known: current_pubkey
						});
					} else if (!nodes.includes((node: any) => node.id === channel.remote_pubkey)) {
						nodes.push({
							id: channel.remote_pubkey,
							alias: channel.remote_pubkey,
							known: current_pubkey
						});
					}
				}
				if (!channel.initiator) {
					return;
				}
				if (edges.includes((edge: any) => edge.channel_id === channel.chan_id)) {
					return;
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
	}

	function map_coreln_channels(
		nodes: any[],
		nodesWeKnow: Record<string, string>,
		edges: any[],
		nodeData: Nodes
	) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'coreln') {
				return;
			}
			let current_pubkey = value.info.id;
			console.log(current_pubkey);
			nodes.push({ id: current_pubkey, alias: key, nodeInfo: value.info, known: current_pubkey });
			value.channels.forEach((channel: any) => {
				console.log('coreln channel', channel);
				let knownNode = nodesWeKnow[channel.id];
				if (!knownNode) {
					if (channel.alias) {
						nodes.push({
							id: channel.id,
							alias: channel.alias,
							known: current_pubkey
						});
					} else if (!nodes.includes((node: any) => node.id === channel.id)) {
						nodes.push({
							id: channel.id,
							alias: channel.id,
							known: current_pubkey
						});
					}
				}
				if (!channel.initiator) {
					return;
				}
				if (edges.includes((edge: any) => edge.channel_id === channel.channel_id)) {
					return;
				}
				edges.push({
					source: current_pubkey,
					target: channel.id,
					channel_id: channel.channel_id,
					capacity: channel.msatoshi_total,
					local_balance: channel.msatoshi_to_them / 1000,
					remote_balance: channel.msatoshi_to_us / 1000,
					initiator: channel.opener === 'local',
					channel: channel
				});
			});
		});
	}

	function map_eclair_channels(
		nodes: any[],
		nodesWeKnow: Record<string, string>,
		edges: any[],
		nodeData: Nodes
	) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'eclair') {
				return;
			}
			let current_pubkey = value.info.nodeId;
			nodes.push({ id: current_pubkey, alias: key, nodeInfo: value.info, known: current_pubkey });
			value.channels.forEach((channel: any) => {
				console.log('eclair channel', channel);
				let knownNode = nodesWeKnow[channel.nodeId];
				if (!knownNode) {
					if (channel.alias) {
						nodes.push({
							id: channel.id,
							alias: channel.alias,
							known: current_pubkey
						});
					} else if (!nodes.includes((node: any) => node.id === channel.nodeId)) {
						nodes.push({
							id: channel.nodeId,
							alias: channel.nodeId,
							known: current_pubkey
						});
					}
				}
				if (!channel.initiator) {
					return;
				}
				if (edges.includes((edge: any) => edge.channel_id === channel.channelId)) {
					return;
				}
				edges.push({
					source: current_pubkey,
					target: channel.nodeId,
					channel_id: channel.channelId,
					capacity: channel.data.commitments.active[0].fundingTx.amountSatoshis,
					local_balance: 0,
					remote_balance: 0, // TODO fix these and see what happens when multiple payments are sent
					initiator: channel.data.commitments.params.localParams.isInitiator,
					channel: channel
				});
			});
		});
	}

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
		const data = event.detail;
		if (data.type == 'channel') {
			jsonData = data.channel;
		} else if (data.type == 'node') {
			if (data.id != data.known) {
				let connection = nodeConnections.find((connection) => connection.pubkey == data.known);
				connection?.connection.fetchSpecificNodeInfo(data.id).then((nodeInfo) => {
					jsonData = nodeInfo;
				});
			} else {
				jsonData = data.nodeInfo;
			}
		} else {
			console.error('data type not supported', data);
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
