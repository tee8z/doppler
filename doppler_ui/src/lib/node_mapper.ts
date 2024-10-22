interface Node {
	id: string;
	alias: string;
	known: string;
	type?: string;
}

interface Channel {
	source: string;
	target: string;
	channel_id: string;
	capacity: number;
	local_balance: number;
	remote_balance: number;
	initiator: boolean;
	active: boolean;
	channel: any;
	types?: string[];
}

interface NodeConnection {
	pubkey: string;
	alias: string;
	connection: any;
}

interface Nodes {
	[key: string]: any;
}

export class ChannelMapper {
	private nodeMap: Map<string, Node>;
	private channelMap: Map<string, Channel>;
	private uniqueNodes: Set<string>;
	private uniqueChannels: Set<string>;
	private nodeConnections: NodeConnection[];
	private nodes: Node[];
	private edges: Channel[];

	constructor() {
		this.nodeMap = new Map();
		this.channelMap = new Map();
		this.uniqueNodes = new Set();
		this.uniqueChannels = new Set();
		this.nodeConnections = [];
		this.nodes = [];
		this.edges = [];
	}

	public processNodeData(nodeData: Nodes, nodeConnections: NodeConnection[]) {
		this.clear();
		this.nodeConnections = nodeConnections;

		// Process each implementation type in order
		this.map_lnd_channels(nodeData);
		this.map_coreln_channels(nodeData);
		this.map_eclair_channels(nodeData);

		return {
			cur_nodes: this.nodes,
			cur_edges: this.edges
		};
	}

	private clear() {
		this.nodeMap.clear();
		this.channelMap.clear();
		this.uniqueNodes.clear();
		this.uniqueChannels.clear();
		this.nodes = [];
		this.edges = [];
	}

	private map_lnd_channels(nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'lnd' || !value.online) return;

			const nodeInfo = {
				id: value.info.identity_pubkey,
				alias: key,
				known: value.info.identity_pubkey,
				type: value.type
			};

			if (!this.nodeMap.has(value.info.identity_pubkey)) {
				this.nodeMap.set(value.info.identity_pubkey, nodeInfo);
				this.nodes.push(nodeInfo);
			} else {
				const existingNode = this.nodeMap.get(value.info.identity_pubkey)!;
				existingNode.type = value.type;
			}
			this.uniqueNodes.add(value.info.identity_pubkey);
		});

		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'lnd' || !value.online) return;

			let current_pubkey = value.info.identity_pubkey;
			value.channels.forEach((channel: any) => {
				if (!channel.remote_pubkey || !channel.initiator) return;

				if (!this.nodeMap.has(channel.remote_pubkey)) {
					let known = this.nodeConnections.find((node) => node.pubkey === channel.remote_pubkey);
					const nodeInfo = {
						id: channel.remote_pubkey,
						alias: known?.alias || channel.remote_pubkey,
						known: channel.remote_pubkey
					};
					this.nodeMap.set(channel.remote_pubkey, nodeInfo);
					this.nodes.push(nodeInfo);
				}
				this.uniqueNodes.add(channel.remote_pubkey);

				const channelInfo: Channel = {
					source: current_pubkey,
					target: channel.remote_pubkey,
					channel_id: channel.chan_id,
					capacity: channel.capacity,
					local_balance: channel.local_balance,
					remote_balance: channel.remote_balance,
					initiator: channel.initiator,
					active: channel.active,
					channel: channel,
					types: ['lnd']
				};

				if (!this.channelMap.has(channel.chan_id)) {
					this.channelMap.set(channel.chan_id, channelInfo);
					this.edges.push(channelInfo);
					this.uniqueChannels.add(channel.chan_id);
				} else {
					const existingChannel = this.channelMap.get(channel.chan_id)!;
					if (!existingChannel.types?.includes('lnd')) {
						existingChannel.types = [...(existingChannel.types || []), 'lnd'];
					}
					Object.assign(existingChannel, channelInfo);
				}
			});
		});
	}

	private map_coreln_channels(nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'coreln' || !value.online) return;

			const nodeInfo = {
				id: value.info.id,
				alias: key,
				known: value.info.id,
				type: value.type
			};

			if (!this.nodeMap.has(value.info.id)) {
				this.nodeMap.set(value.info.id, nodeInfo);
				this.nodes.push(nodeInfo);
			} else {
				const existingNode = this.nodeMap.get(value.info.id)!;
				existingNode.type = value.type;
			}
			this.uniqueNodes.add(value.info.id);
		});

		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'coreln' || !value.online) return;

			let current_pubkey = value.info.id;
			value.channels.forEach((channel: any) => {
				if (channel.opener !== 'local') return;

				if (!this.nodeMap.has(channel.id)) {
					let known = this.nodeConnections.find((node) => node.pubkey === channel.id);
					const nodeInfo = {
						id: channel.id,
						alias: known?.alias || channel.id,
						known: current_pubkey
					};
					this.nodeMap.set(channel.id, nodeInfo);
					this.nodes.push(nodeInfo);
				}
				this.uniqueNodes.add(channel.id);

				const channelInfo: Channel = {
					source: current_pubkey,
					target: channel.id,
					channel_id: channel.channel_id,
					capacity: channel.msatoshi_total,
					local_balance: Math.floor(channel.msatoshi_to_us / 1000),
					remote_balance: Math.floor(channel.msatoshi_to_them / 1000),
					initiator: channel.opener === 'local',
					active: channel.state === 'CHANNELD_NORMAL',
					channel: channel,
					types: ['coreln']
				};

				if (!this.channelMap.has(channel.channel_id)) {
					this.channelMap.set(channel.channel_id, channelInfo);
					this.edges.push(channelInfo);
					this.uniqueChannels.add(channel.channel_id);
				} else {
					const existingChannel = this.channelMap.get(channel.channel_id)!;
					if (!existingChannel.types?.includes('coreln')) {
						existingChannel.types = [...(existingChannel.types || []), 'coreln'];
					}
					Object.assign(existingChannel, channelInfo);
				}
			});
		});
	}

	private map_eclair_channels(nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'eclair' || !value.online) return;

			const nodeInfo = {
				id: value.info.nodeId,
				alias: key,
				known: value.info.nodeId,
				type: value.type
			};

			if (!this.nodeMap.has(value.info.nodeId)) {
				this.nodeMap.set(value.info.nodeId, nodeInfo);
				this.nodes.push(nodeInfo);
			} else {
				const existingNode = this.nodeMap.get(value.info.nodeId)!;
				existingNode.type = value.type;
			}
			this.uniqueNodes.add(value.info.nodeId);
		});

		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (value.type !== 'eclair' || !value.online) return;

			let current_pubkey = value.info.nodeId;
			value.channels.forEach((channel: any) => {
				if (!channel.data.commitments.params.localParams.isInitiator) return;

				if (!this.nodeMap.has(channel.nodeId)) {
					let known = this.nodeConnections.find((node) => node.pubkey === channel.nodeId);
					const nodeInfo = {
						id: channel.nodeId,
						alias: known?.alias || channel.nodeId,
						known: current_pubkey
					};
					this.nodeMap.set(channel.nodeId, nodeInfo);
					this.nodes.push(nodeInfo);
				}
				this.uniqueNodes.add(channel.nodeId);

				const channelInfo: Channel = {
					source: current_pubkey,
					target: channel.nodeId,
					channel_id: channel.channelId,
					capacity: channel.data.commitments.active[0].fundingTx.amountSatoshis,
					local_balance: Math.floor(
						channel.data.commitments.active[0].localCommit.spec.toLocal / 1000
					),
					remote_balance: Math.floor(
						channel.data.commitments.active[0].localCommit.spec.toRemote / 1000
					),
					initiator: channel.data.commitments.params.localParams.isInitiator,
					active: channel.state === 'NORMAL',
					channel: channel,
					types: ['eclair']
				};

				if (!this.channelMap.has(channel.channelId)) {
					this.channelMap.set(channel.channelId, channelInfo);
					this.edges.push(channelInfo);
					this.uniqueChannels.add(channel.channelId);
				} else {
					const existingChannel = this.channelMap.get(channel.channelId)!;
					if (!existingChannel.types?.includes('eclair')) {
						existingChannel.types = [...(existingChannel.types || []), 'eclair'];
					}
					Object.assign(existingChannel, channelInfo);
				}
			});
		});
	}
}
