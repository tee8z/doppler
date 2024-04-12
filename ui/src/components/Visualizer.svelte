<script lang="ts">
	import { LndRequests } from '$lib/lnd_requests';
	import { onDestroy, onMount, tick } from 'svelte';
	import Graph from './Graph.svelte';
	import { edges, nodes } from './Graph.svelte';
	import Info from './Info.svelte';
	import Button from './Button.svelte';
	import type { Connections } from '../routes/api/connections/+server';
	import { getConnections } from '$lib/connections';
	import type { NodeRequests, Nodes } from '$lib/nodes';
	import { CorelnRequests } from '$lib/coreln_requests';
	import { EclairRequests } from '$lib/eclair_requests';
	let dataPromise: Promise<Nodes> | null = null;
	let poller: ReturnType<typeof setInterval>;
	let isPolling = false;
	let connections: Connections;
	let info: any = null;
	let jsonData: any = null;
	let nodeConnections: { pubkey: string; alias: string; connection: NodeRequests }[] = [];
	function setNode(node: any) {
		info = node.info;
		jsonData = node.info;
	}
	//TODO: add color to channels to show active/inactive
	//TODO: show a light or symbol when each given node sees all the channels, use graph description for this
	const fetchData = async (connections: Connections) => {
		let nodeData: Nodes = {};
		const promises = Object.keys(connections).map(async (key) => {
			const connectionConfig = connections[key];
			if (connectionConfig.type === 'lnd') {
				let requests = new LndRequests(
					connectionConfig.host,
					connectionConfig.macaroon,
				);
				let response = await requests.fetchChannels();
				let channels = [];
				if (response && response.channels) {
					channels = response.channels;
				}
				let balance = await requests.fetchBalance();
				let info = await requests.fetchInfo();
				if (!response['error']) {
					nodeConnections.push({ pubkey: info.identity_pubkey, alias: key, connection: requests });
				}
				if (channels && balance && info) {
					return {
						[key]: {
							channels,
							balance,
							info,
							type: 'lnd',
							online: response['error'] ? false : true
						}
					};
				}
			} else if (connectionConfig.type === 'coreln') {
				const requests = new CorelnRequests(connectionConfig.host, connectionConfig.macaroon);
				const channels = await requests.fetchChannels();
				const balance = await requests.fetchBalance();
				const info = await requests.fetchInfo();
				if (!info['error']) {
					nodeConnections.push({ pubkey: info.id, alias: key, connection: requests });
				}
				if (channels && balance && info) {
					return {
						[key]: {
							channels,
							balance,
							info,
							type: 'coreln',
							online: info['error'] ? false : true
						}
					};
				}
			} else if (connectionConfig.type === 'eclair') {
				const requests = new EclairRequests(connectionConfig.host, connectionConfig.password);
				const channels = await requests.fetchChannels();
				const balance = await requests.fetchBalance();
				const info = await requests.fetchInfo();
				if (!info['error']) {
					nodeConnections.push({ pubkey: info.nodeId, alias: key, connection: requests });
				}
				if (channels && balance && info) {
					return {
						[key]: {
							channels,
							balance,
							info,
							type: 'eclair',
							online: info['error'] ? false : true
						}
					};
				}
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

		let cur_nodes: any[] = [];
		let cur_edges: any[] = [];
		map_lnd_channels(cur_nodes, cur_edges, nodeData);
		map_coreln_channels(cur_nodes, cur_edges, nodeData);
		map_eclair_channels(cur_nodes, cur_edges, nodeData);
		nodes.set(cur_nodes);
		edges.set(cur_edges);
		dataPromise = Promise.resolve(nodeData);
	};

	function map_lnd_channels(nodes: any[], edges: any[], nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'lnd') {
				return;
			}
			if (!value.online) {
				return;
			}
			if (!has_node(nodes, value.info.identity_pubkey)) {
				nodes.push({
					id: value.info.identity_pubkey,
					alias: key,
					known: value.info.identity_pubkey
				});
			}
		});
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'lnd') {
				return;
			}
			if (!value.online) {
				return;
			}
			let current_pubkey = value.info.identity_pubkey;
			value.channels.forEach((channel: any) => {
				if (!channel.remote_pubkey) {
					return;
				}
				if (!has_node(nodes, channel.remote_pubkey)) {
					let known = nodeConnections.find((node) => node.pubkey === channel.remote_pubkey);
					nodes.push({
						id: channel.remote_pubkey,
						alias: known && known.alias ? known.alias : channel.remote_pubkey,
						known: channel.remote_pubkey
					});
				}
				if (!channel.initiator) {
					return;
				}
				if (has_channel(edges, channel.chan_id)) {
					return;
				}
				console.log(channel);
				edges.push({
					source: current_pubkey,
					target: channel.remote_pubkey,
					channel_id: channel.chan_id,
					capacity: channel.capacity,
					local_balance: channel.local_balance,
					remote_balance: channel.remote_balance,
					initiator: channel.initiator,
					active: channel.active,
					channel: channel
				});
			});
		});
	}

	function has_node(nodes: any[], channel_remote: string) {
		for (let node of nodes) {
			if (node.id === channel_remote) {
				return true;
			}
		}
		return false;
	}

	function has_channel(channels: any[], channel_id: string) {
		for (let channel of channels) {
			if (channel.channel_id === channel_id) {
				return true;
			}
		}
		return false;
	}

	function map_coreln_channels(nodes: any[], edges: any[], nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'coreln') {
				return;
			}
			if (!value.online) {
				return;
			}
			if (!has_node(nodes, value.info.id)) {
				nodes.push({
					id: value.info.id,
					alias: key,
					known: value.info.id
				});
			}
		});
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'coreln') {
				return;
			}
			if (!value.online) {
				return;
			}
			let current_pubkey = value.info.id;
			value.channels.forEach((channel: any) => {
				if (!has_node(nodes, channel.id)) {
					let known = nodeConnections.find((node) => node.pubkey === channel.id);
					nodes.push({
						id: channel.id,
						alias: (known && known.alias) ? known.alias : channel.id,
						known: current_pubkey
					});
				}
				if (!(channel.opener === 'local')) {
					return;
				}
				if (has_channel(edges, channel.channel_id)) {
					return;
				}
				console.log(channel);
				let edge = {
					source: current_pubkey,
					target: channel.id,
					channel_id: channel.channel_id,
					capacity: channel.msatoshi_total,
					local_balance: Math.floor(channel.msatoshi_to_us / 1000),
					remote_balance: Math.floor(channel.msatoshi_to_them / 1000),
					initiator: channel.opener === 'local',
					active: channel.state === 'CHANNELD_NORMAL',
					channel: channel
				};
				edges.push(edge);
			});
		});
	}

	function map_eclair_channels(nodes: any[], edges: any[], nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'eclair') {
				return;
			}
			if (!value.online) {
				return;
			}
			if (!has_node(nodes, value.info.nodeId)) {
				nodes.push({
					id: value.info.nodeId,
					alias: key,
					known: value.info.nodeId
				});
			}
		});
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'eclair') {
				return;
			}
			if (!value.online) {
				return;
			}
			let current_pubkey = value.info.nodeId;
			value.channels.forEach((channel: any) => {
				if (!has_node(nodes, channel.nodeId)) {
					let known = nodeConnections.find((node) => node.pubkey === channel.nodeId);
					nodes.push({
						id: channel.nodeId,
						alias: (known && known.alias) ? known.alias : channel.nodeId,
						known: current_pubkey
					});
				}
				if (!channel.data.commitments.params.localParams.isInitiator) {
					return;
				}
				if (has_channel(edges, channel.channelId)) {
					return;
				}
				console.log(channel);
				let edge = {
					source: current_pubkey,
					target: channel.nodeId,
					channel_id: channel.channelId,
					capacity: channel.data.commitments.active[0].fundingTx.amountSatoshis,
					local_balance: Math.floor(channel.data.commitments.active[0].localCommit.spec.toLocal / 1000),
					remote_balance: Math.floor(channel.data.commitments.active[0].localCommit.spec.toRemote / 1000), // TODO fix these and see what happens when multiple payments are sent
					initiator: channel.data.commitments.params.localParams.isInitiator,
					active: channel.state === 'NORMAL',
					channel: channel
				};
				edges.push(edge);
			});
		});
	}

	onMount(async () => {
		if (poller) {
			clearInterval(poller);
		}
		tick();
		connections = await getConnections();
		fetchData(connections);
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

	function stop() {
		isPolling = false;
		if (poller) {
			clearInterval(poller);
		}
	}

	function start() {
		if (connections) {
			poller = setInterval(() => fetchData(connections), 15000); // Poll every 15 seconds
			isPolling = true;
		}
	}
</script>

{#await dataPromise}
	<p>Loading graph...</p>
{:then nodeData}
	<div class="info">
		<div>
			<h1>Visualize</h1>
			<div>
				<span>Polling</span>
				<Button on:click={start}>Start</Button>
				<Button on:click={stop}>Stop</Button>
				<label class="switch">
					<input type="checkbox" id="pollingToggle" bind:checked={isPolling} />
					<span class="slider round" />
				</label>
			</div>
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
		{#if $nodes.length > 0}
			<Graph on:dataEvent={handleClickData} />
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
	.switch {
		position: relative;
		display: inline-block;
		width: 60px;
		height: 34px;
	}

	.switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.slider {
		position: absolute;
		cursor: pointer;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: #ccc;
		transition: 0.4s;
	}

	.slider:before {
		position: absolute;
		content: '';
		height: 26px;
		width: 26px;
		left: 4px;
		bottom: 4px;
		background-color: white;
		transition: 0.4s;
	}

	input:checked + .slider {
		background-color: #2196f3;
	}

	input:checked + .slider:before {
		transform: translateX(26px);
	}

	.slider.round {
		border-radius: 34px;
	}

	.slider.round:before {
		border-radius: 50%;
	}
</style>
