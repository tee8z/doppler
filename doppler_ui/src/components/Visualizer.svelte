<script lang="ts">
	import { LndRequests } from '$lib/lnd_requests';
	import { onDestroy, onMount, tick } from 'svelte';
	import Graph from './Graph.svelte';
	import Select from './Select.svelte';
	import { edges, nodes } from './Graph.svelte';
	import Info from './Info.svelte';
	import Button from './Button.svelte';
	import ResizablePanel from './ResizablePanel.svelte';
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

	// Separate state for node and channel info
	let nodeInfo: any = null;
	let nodeBalance: any = null;
	let nodeType: any = null;
	let channelInfo: any = null;
	let currentNodeId: string | null = null;

	// Combined info for display
	let info: any = null;
	let balance: any = null;
	let type: any = null;

	let jsonData: any = null;
	let currentData: any = null;
	let selectedView: 'all' | 'source' | 'target' = 'all';
	let dataType: 'node' | 'channel' | null = null;

	let nodeConnections: { pubkey: string; alias: string; connection: NodeRequests }[] = [];

	async function setNode(node: any, isChannelNode: boolean = false) {
		if (!node) return;

		// Store node information
		nodeInfo = node.info;
		nodeBalance = node.balance;
		nodeType = node.type;
		currentNodeId = node.info?.identity_pubkey || node.info?.id || node.info?.nodeId;

		// Update display info
		info = nodeInfo;
		balance = nodeBalance;
		type = nodeType;

		// Only update JSON data if this isn't a channel-related node update
		if (!isChannelNode) {
			jsonData = nodeInfo;
			currentData = node;
			dataType = 'node';
			channelInfo = null;
		}
	}

	function updateJsonData() {
		if (!currentData) return;

		if (dataType === 'node') {
			jsonData = nodeInfo;
			info = nodeInfo;
			type = nodeType;
			balance = nodeBalance;
		} else if (dataType === 'channel') {
			if (selectedView === 'all') {
				jsonData = currentData;
			} else {
				const perspective = currentData.find((d: any) => d.perspective === selectedView);
				jsonData = perspective || null;
			}

			info = nodeInfo;
			type = nodeType;
			balance = nodeBalance;
		}
	}

	const fetchData = async (cur_connections: Connections) => {
		if (!cur_connections || Object.keys(cur_connections).length === 0) {
			console.log('No connections provided. Aborting fetchData.');

			nodeConnections = [];
			nodes.set([]);
			edges.set([]);
			nodeInfo = null;
			nodeBalance = null;
			nodeType = null;
			channelInfo = null;
			currentNodeId = null;
			info = null;
			balance = null;
			type = null;
			jsonData = null;
			currentData = null;
			dataType = null;
			dataPromise = null;
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
					nodeConnections.push({
						pubkey: info.identity_pubkey,
						alias: key,
						connection: requests
					});
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
				const requests = new EclairRequests(
					connectionConfig.host,
					connectionConfig.password
				);
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

	async function handleClickData(event: any) {
		const data = event.detail;

		if (!data) return;

		if (data.type === 'channel') {
			// First update node info if needed
			await updateNodeInfoForChannel(data.channel);

			// Then set channel info
			setChannel(data.channel);
		} else if (data.type === 'node') {
			dataType = 'node';
			currentData = data;

			if (data.known) {
				const connection = nodeConnections.find(
					(connection) => connection.pubkey === data.known
				);
				if (connection) {
					try {
						const nodeInfo = await connection.connection.fetchInfo();
						currentData.info = nodeInfo;
						await setNode(currentData);
					} catch (error) {
						console.error('Error fetching node info:', error);
					}
				}
			} else if (data.id !== data.known) {
				const connection = nodeConnections.find(
					(connection) => connection.pubkey === data.known
				);
				if (connection) {
					try {
						const nodeInfo = await connection.connection.fetchSpecificNodeInfo(data.id);
						currentData.info = nodeInfo;
						await setNode(currentData);
					} catch (error) {
						console.error('Error fetching specific node info:', error);
					}
				}
			} else {
				await setNode(data);
			}
		} else {
			console.error('Unsupported data type:', data);
		}

		updateJsonData();
	}

	async function fetchAndSetNodeInfo(
		nodeId: string | null,
		isChannelNode: boolean = false
	): Promise<boolean> {
		if (!nodeId) return false;

		// Find the connection that can fetch this node's info
		for (const connection of nodeConnections) {
			try {
				let nodeInfo;
				if (connection.pubkey === nodeId) {
					nodeInfo = await connection.connection.fetchInfo();
				} else {
					nodeInfo = await connection.connection.fetchSpecificNodeInfo(nodeId);
				}
				if (nodeInfo) {
					await setNode(
						{
							info: nodeInfo,
							type: connection.connection.constructor.name.toLowerCase(),
							balance: null
						},
						isChannelNode
					);
					return true;
				}
			} catch (error) {
				console.error(
					`Error fetching node info using connection ${connection.alias}:`,
					error
				);
			}
		}
		return false;
	}

	function setChannel(channel: any) {
		if (!channel) return;

		currentData = channel;
		dataType = 'channel';

		// Store channel information
		if (selectedView === 'all') {
			channelInfo = channel;
		} else {
			const perspective = channel.find((d: any) => d.perspective === selectedView);
			channelInfo = perspective || null;
		}

		// Update JSON display with channel info
		jsonData = channelInfo;
	}

	async function updateNodeInfoForChannel(channelData: any) {
		const { sourceNodeId, targetNodeId } = extractNodeIds(channelData);

		// If current node is neither source nor target, update to most appropriate node
		if (currentNodeId !== sourceNodeId && currentNodeId !== targetNodeId) {
			// Try source node first
			const sourceSuccess = await fetchAndSetNodeInfo(sourceNodeId, true);
			if (!sourceSuccess) {
				// Fall back to target node if source fails
				await fetchAndSetNodeInfo(targetNodeId, true);
			}
		}
	}

	function extractNodeIds(channelData: any): {
		sourceNodeId: string | null;
		targetNodeId: string | null;
	} {
		let sourceNodeId = null;
		let targetNodeId = null;

		if (Array.isArray(channelData)) {
			const sourceView = channelData.find((d: any) => d.perspective === 'source');
			const targetView = channelData.find((d: any) => d.perspective === 'target');

			sourceNodeId = sourceView?.node_id || sourceView?.nodeId;
			targetNodeId = targetView?.node_id || targetView?.nodeId;
		} else {
			sourceNodeId = channelData.source_node_id || channelData.sourceNodeId;
			targetNodeId = channelData.target_node_id || channelData.targetNodeId;
		}

		return { sourceNodeId, targetNodeId };
	}
	function prettyPrintJson(jsonData: any) {
		return JSON.stringify(jsonData, null, 2);
	}

	function prettyPrintConnections(connections: Connections): string {
		const filteredConnections = Object.entries(connections).reduce(
			(acc, [key, config]) => {
				const filteredConfig = Object.entries(config).reduce(
					(configAcc, [propKey, propValue]) => {
						if (propValue != null && propValue !== '') {
							if (isKeyOfConnectionConfig(propKey)) {
								configAcc[propKey] = propValue;
							}
						}
						return configAcc;
					},
					{} as Partial<ConnectionConfig>
				);

				acc[key] = filteredConfig;
				return acc;
			},
			{} as { [key: string]: Partial<ConnectionConfig> }
		);

		return JSON.stringify(filteredConnections, null, 2);
	}

	function isKeyOfConnectionConfig(key: string): key is keyof ConnectionConfig {
		return [
			'macaroon',
			'password',
			'user',
			'rune',
			'host',
			'type',
			'rpc_port',
			'p2p_port'
		].includes(key);
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
		<ResizablePanel
			defaultWidth={400}
			leftPanelBackground="rgba(0, 151, 19, 0.1)"
			rightPanelBackground="white"
			zIndex={1}
		>
			<div slot="left" class="info-panel">
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
						<span class="slider round"></span>
					</label>
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
			<div slot="right" class="graph-container">
				{#if $nodes && $nodes.length > 0}
					<Graph on:dataEvent={handleClickData} />
				{/if}
			</div>
		</ResizablePanel>
	{:catch error}
		<p>Error: {error.message}</p>
	{/await}
</div>

<style>
	.visualizer {
		@apply bg-white dark:bg-gray-900 text-gray-900 dark:text-white;
		position: relative;
		height: 100%;
		width: 100%;
		overflow: hidden;
	}

	.info-panel {
		@apply bg-green-50/10 dark:bg-gray-800/20;
		padding: 10px;
		font-size: large;
		overflow-y: auto;
		height: 100%;
		width: 100%;
	}

	.graph-container {
		height: 100%;
		width: 100%;
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
