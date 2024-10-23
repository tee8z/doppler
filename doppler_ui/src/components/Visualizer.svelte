<script lang="ts">
	import { LndRequests } from '$lib/lnd_requests';
	import { onDestroy, onMount, tick } from 'svelte';
	import Graph from './Graph.svelte';
	import Select from './Select.svelte';
	import { edges, nodes } from './Graph.svelte';
	import Info from './Info.svelte';
	import Button from './Button.svelte';
	import type { ConnectionConfig, Connections } from '$lib/connections';
	import { getConnections } from '$lib/connections';
	import type { NodeRequests, Nodes } from '$lib/nodes';
	import { CorelnRequests } from '$lib/coreln_requests';
	import { EclairRequests } from '$lib/eclair_requests';
	import { ChannelMapper } from '$lib/node_mapper';

	let dataPromise: Promise<Nodes> | null = null;
	let poller: ReturnType<typeof setInterval>;
	let isPolling = false;
	let connections: Connections;
	let showConnections = false;
	let info: any = null;
	let jsonData: any = null;
	let currentData: any = null;
	let selectedView: 'all' | 'source' | 'target' = 'all';
	let dataType: 'node' | 'channel' | null = null;

	let nodeConnections: { pubkey: string; alias: string; connection: NodeRequests }[] = [];

	function setNode(node: any) {
		if (!node) {
			return;
		}
		info = node.info;
		jsonData = node.info;
		currentData = node.info;
		dataType = 'node';
	}

	function updateJsonData() {
		if (!currentData || dataType === 'node') {
			jsonData = currentData;
			return;
		}

		if (dataType === 'channel') {
			if (selectedView === 'all') {
				jsonData = currentData;
			} else {
				const perspective = currentData.find((d: any) => d.perspective === selectedView);
				jsonData = perspective ? perspective : null;
			}
		}
	}

	const fetchData = async (cur_connections: Connections) => {
		if (!cur_connections || Object.keys(cur_connections).length === 0) {
			console.log('No connections provided. Aborting fetchData.');
			return;
		}
		let nodeData: Nodes = {};
		nodeConnections = [];

		const promises = Object.keys(cur_connections).map(async (key) => {
			const connectionConfig = cur_connections[key];
			if (connectionConfig.type === 'lnd') {
				let requests = new LndRequests(connectionConfig.host, connectionConfig.macaroon);
				let response = await requests.fetchChannels();
				let channels = [];
				if (response && response.channels) {
					channels = response.channels;
				}
				let balance = await requests.fetchBalance();
				let info = await requests.fetchInfo();
				if (!response['error'] && info && info.identity_pubkey && key) {
					nodeConnections.push({ pubkey: info.identity_pubkey, alias: key, connection: requests });
				}
				if (channels && balance && info && key) {
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
				const requests = new CorelnRequests(connectionConfig.host, connectionConfig.rune);
				const channels = await requests.fetchChannels();
				const balance = await requests.fetchBalance();
				const info = await requests.fetchInfo();
				if (!info['error'] && info && info.id && key) {
					nodeConnections.push({ pubkey: info.id, alias: key, connection: requests });
				}
				if (channels && balance && info && key) {
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
				if (!info['error'] && info && info.nodeId && key) {
					nodeConnections.push({ pubkey: info.nodeId, alias: key, connection: requests });
				}
				if (
					channels &&
					!channels.error &&
					balance &&
					!balance.error &&
					info &&
					!info.error &&
					key
				) {
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
				console.warn('issue building the requests client');
				return;
			}
			const [key, data] = Object.entries(result)[0];
			if (key && data) {
				nodeData[key] = data;
			}
		});

		// Set initial node data
		let key = Object.keys(nodeData)[0];
		if (key && (!info || !jsonData)) {
			setNode(nodeData[key]);
		}

		const channelMapper = new ChannelMapper();
		// use to see what the object look like across the node: console.log(JSON.stringify(nodeData));
		const { cur_nodes, cur_edges } = channelMapper.processNodeData(nodeData, nodeConnections);

		nodes.set(cur_nodes);
		edges.set(cur_edges);
		dataPromise = Promise.resolve(nodeData);
	};

	onMount(async () => {
		if (poller) {
			clearInterval(poller);
		}
		tick();
	});

	onDestroy(() => {
		if (poller) {
			clearInterval(poller);
		}
	});
	function handleClickData(event: any) {
		const data = event.detail;

		if (data.type === 'channel') {
			currentData = data.channel;
			dataType = 'channel';
			updateJsonData();
		} else if (data.type === 'node') {
			dataType = 'node';
			if (data.known) {
				let connection = nodeConnections.find((connection) => connection.pubkey == data.known);
				connection?.connection.fetchInfo().then((nodeInfo) => {
					currentData = nodeInfo;
					updateJsonData();
				});
			} else if (data.id != data.known) {
				let connection = nodeConnections.find((connection) => connection.pubkey == data.known);

				connection?.connection.fetchSpecificNodeInfo(data.id).then((nodeInfo) => {
					currentData = nodeInfo;
					updateJsonData();
				});
			}
		} else {
			console.error('data type not supported', data);
		}
	}

	function prettyPrintJson(jsonData: any) {
		return JSON.stringify(jsonData, null, 2);
	}

	function prettyPrintConnections(connections: Connections): string {
		const filteredConnections = Object.entries(connections).reduce(
			(acc, [key, config]) => {
				const filteredConfig = Object.entries(config).reduce((configAcc, [propKey, propValue]) => {
					if (propValue != null && propValue !== '') {
						if (isKeyOfConnectionConfig(propKey)) {
							configAcc[propKey] = propValue;
						}
					}
					return configAcc;
				}, {} as Partial<ConnectionConfig>);

				acc[key] = filteredConfig;
				return acc;
			},
			{} as { [key: string]: Partial<ConnectionConfig> }
		);

		return JSON.stringify(filteredConnections, null, 2);
	}

	function isKeyOfConnectionConfig(key: string): key is keyof ConnectionConfig {
		return ['macaroon', 'password', 'rune', 'host', 'type'].includes(key);
	}

	function stop() {
		isPolling = false;
		if (poller) {
			clearInterval(poller);
		}
	}

	function togglePolling() {
		if (isPolling) {
			stop();
		} else {
			start();
		}
	}

	async function fetchAndUpdateData() {
		connections = await getConnections();
		await fetchData(connections);
	}

	function start() {
		if (isPolling) {
			console.log('Polling is already active. Fetching most recent data...');
			fetchAndUpdateData()
				.then(() => {
					console.log('Got data outside of polling');
				})
				.catch((error) => {
					console.error('Error in manual data fetch:', error);
				});
			return;
		}

		isPolling = true;

		// Perform immediate call
		fetchAndUpdateData()
			.then(() => {
				if (poller) {
					clearInterval(poller);
				}
				poller = setInterval(fetchAndUpdateData, 15000); // Poll every 15 seconds
			})
			.catch((error) => {
				console.error('Error in initial data fetch:', error);
				isPolling = false;
			});
	}

	$: jsonData, selectedView, updateJsonData();
</script>

<div class="visualizer">
	{#await dataPromise}
		<p>Loading graph...</p>
	{:then nodeData}
		<div class="info-panel">
			<h1>Visualize</h1>
			<div>
				<span>Polling</span>
				<Button on:click={start}>Start</Button>
				<Button on:click={stop}>Stop</Button>
				<Button on:click={() => (showConnections = !showConnections)}>
					{showConnections ? 'Hide' : 'Show'} Connections
				</Button>
				<label class="switch">
					<input
						type="checkbox"
						id="pollingToggle"
						bind:checked={isPolling}
						on:change={togglePolling}
					/>
					<span class="slider round"> </span></label
				>
			</div>
			{#if showConnections && connections}
				<div class="connections-container">
					<h2>Connection Details</h2>
					<pre class="connections">{prettyPrintConnections(connections)}</pre>
				</div>
			{/if}
			<Info {info} />

			{#if dataType === 'channel'}
				<div class="view-selector">
					<Select
						bind:value={selectedView}
						options={[
							{ value: 'all', label: 'View All' },
							{ value: 'source', label: 'Source View' },
							{ value: 'target', label: 'Target View' }
						]}
					/>
				</div>
			{/if}

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
		<div class="graph-container">
			{#if $nodes && $nodes.length > 0}
				<Graph on:dataEvent={handleClickData} />
			{/if}
		</div>
	{:catch error}
		<p>Error: {error.message}</p>
	{/await}
</div>

<style>
	.visualizer {
		display: flex;
		height: 100vh;
		overflow: hidden;
	}
	.info-panel {
		flex: 0 0 30%;
		max-width: 400px;
		background: rgba(0, 151, 19, 0.1);
		padding: 10px;
		font-size: large;
		overflow-y: auto;
		height: 100%;
	}
	.graph-container {
		flex: 1;
		height: 100%;
		display: flex;
		justify-content: center;
		align-items: center;
		overflow: hidden;
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
