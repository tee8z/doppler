interface Node {
	id: string;
	alias: string;
	known: string;
	type?: string;
	info?: any;
	balance?: any;
}

interface ChannelData {
	type: string;
	alias?: string;
	channel: any;
	perspective: 'source' | 'target';
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
	data: ChannelData[];
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

		// Map all known nodes first
		this.mapAllNodes(nodeData);

		// Then map all channels and discover additional nodes
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (!value.online) return;
			this.mapChannels(value.type, key, value);
		});

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

	private addNode(nodeId: string, alias: string, type?: string, value: any) {
		if (!this.nodeMap.has(nodeId)) {
			// First try to find a known connection
			const known = this.nodeConnections.find((node) => node.pubkey === nodeId);

			const nodeInfo: Node = {
				id: nodeId,
				alias: known?.alias || alias, // Use known alias if available, otherwise use provided alias
				known: nodeId,
				info: value?.info,
				balance: value?.balance
			};

			if (type) {
				nodeInfo.type = type;
			}

			this.nodeMap.set(nodeId, nodeInfo);
			this.nodes.push(nodeInfo);
			this.uniqueNodes.add(nodeId);
		} else if (type) {
			// If node exists and type is provided, update the type
			const existingNode = this.nodeMap.get(nodeId)!;
			existingNode.type = type;
		}
	}

	private mapAllNodes(nodeData: Nodes) {
		Object.entries(nodeData).forEach(([key, value]: [string, any]) => {
			if (!value.online) return;

			const nodeId = this.getNodeId(value);
			if (!nodeId) return;
			this.addNode(nodeId, key, value.type, value);
		});
	}

	private getNodeId(node: any): string | null {
		switch (node.type) {
			case 'lnd':
				return node.info?.identity_pubkey || null;
			case 'coreln':
				return node.info?.id || null;
			case 'eclair':
				return node.info?.nodeId || null;
			default:
				return null;
		}
	}

	private getNodeAlias(pubkey: string): string {
		// Check our node map first
		const node = this.nodeMap.get(pubkey);
		if (node?.alias) {
			return node.alias;
		}

		// Check node connections
		const connection = this.nodeConnections.find((n) => n.pubkey === pubkey);
		if (connection?.alias) {
			return connection.alias;
		}

		// Fallback to pubkey if no alias found
		return pubkey;
	}

	private getChannelIdentifiers(
		type: string,
		channel: any
	): {
		channelId: string | null;
		txid: string | null;
	} {
		switch (type) {
			case 'eclair': {
				const channelId = channel.data?.commitments?.params?.channelId || null;
				const outPoint = channel.data?.commitments?.active[0]?.fundingTx?.outPoint || null;
				const txid = outPoint?.split(':')[0] || null;
				return { channelId, txid };
			}
			case 'coreln': {
				return {
					channelId: channel.channel_id || null,
					txid: channel.funding_txid || null
				};
			}
			case 'lnd': {
				const channelPoint = channel.channel_point || null;
				const txid = channelPoint?.split(':')[0] || null;
				return {
					channelId: channel.chan_id || null,
					txid
				};
			}
			default:
				return { channelId: null, txid: null };
		}
	}

	private mapChannels(type: string, nodeKey: string, nodeData: any) {
		const currentPubkey = this.getNodeId(nodeData);
		if (!currentPubkey) return;
		const channels = this.getChannels(type, nodeData);
		if (!channels) return;

		channels.forEach((channel: any) => {
			const { channelId, txid } = this.getChannelIdentifiers(type, channel);
			if (!channelId && !txid) return;

			const remotePubkey = this.getRemotePubkey(type, channel);
			if (!remotePubkey) return;

			// Try to find existing channel
			const existingChannel = Array.from(this.channelMap.entries()).find(([_, ch]) => {
				return ch.data.some((d) => {
					const ids = this.getChannelIdentifiers(d.type, d.channel);

					// First try to match by channel ID if available
					if (channelId && ids.channelId === channelId) {
						return true;
					}

					// Fall back to matching by transaction ID
					if (txid && ids.txid === txid) {
						return true;
					}

					return false;
				});
			});

			// Use channel ID if available, otherwise use txid
			const consistentChannelId = existingChannel ? existingChannel[0] : channelId || txid;

			if (!consistentChannelId) return;

			const currentAlias = this.getNodeAlias(currentPubkey);
			const isInitiator = this.isChannelInitiator(type, channel);
			const isActive = this.isChannelActive(type, channel);

			const channelData: ChannelData = {
				type,
				alias: currentAlias,
				channel,
				perspective: isInitiator ? 'source' : 'target'
			};

			if (!this.channelMap.has(consistentChannelId)) {
				const channelInfo: Channel = {
					source: isInitiator ? currentPubkey : remotePubkey,
					target: isInitiator ? remotePubkey : currentPubkey,
					channel_id: channelId || txid || '',
					capacity: this.getChannelCapacity(type, channel),
					local_balance: this.getLocalBalance(type, channel),
					remote_balance: this.getRemoteBalance(type, channel),
					initiator: isInitiator,
					active: isActive,
					data: [channelData]
				};
				this.channelMap.set(consistentChannelId, channelInfo);
				this.edges.push(channelInfo);
			} else {
				const existingChannel = this.channelMap.get(consistentChannelId)!;

				// Check if we already have data from this node
				const hasNodeData = existingChannel.data.some(
					(d) => d.type === type && d.alias === currentAlias
				);

				// Only add if we don't have this node's view yet
				if (!hasNodeData) {
					existingChannel.data.push(channelData);
				}

				// Update properties
				existingChannel.active = existingChannel.active && isActive;

				// Update balances from initiator's perspective
				if (isInitiator) {
					existingChannel.capacity = this.getChannelCapacity(type, channel);
					existingChannel.local_balance = this.getLocalBalance(type, channel);
					existingChannel.remote_balance = this.getRemoteBalance(type, channel);
				}

				// If we now have a channel ID, update from txid
				if (channelId && existingChannel.channel_id === txid) {
					existingChannel.channel_id = channelId;
				}
			}
		});
	}

	private getChannels(type: string, nodeData: any): any[] | null {
		return nodeData.channels || null;
	}

	private getRemotePubkey(type: string, channel: any): string | null {
		switch (type) {
			case 'lnd':
				return channel.remote_pubkey;
			case 'coreln':
				return channel.peer_id;
			case 'eclair':
				return channel.nodeId;
			default:
				return null;
		}
	}

	private isChannelInitiator(type: string, channel: any): boolean {
		switch (type) {
			case 'lnd':
				return channel.initiator;
			case 'coreln':
				return channel.opener === 'local';
			case 'eclair':
				return channel.data?.commitments?.params?.localParams?.isInitiator || false;
			default:
				return false;
		}
	}

	private getChannelCapacity(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.capacity) || 0;
			case 'coreln':
				return channel.amount_msat ? Math.floor(channel.amount_msat / 1000) : 0;
			case 'eclair':
				return channel.data?.commitments?.active[0]?.fundingTx?.amountSatoshis || 0;
			default:
				return 0;
		}
	}

	private getLocalBalance(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.local_balance) || 0;
			case 'coreln':
				return channel.to_us_msat ? Math.floor(channel.to_us_msat / 1000) : 0;
			case 'eclair':
				return Math.floor(
					(channel.data?.commitments?.active[0]?.localCommit?.spec?.toLocal || 0) / 1000
				);
			default:
				return 0;
		}
	}
	//to_us_msat (msat, optional): How much of channel is owed to us.

	private getRemoteBalance(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.remote_balance) || 0;
			case 'coreln': {
				const total = channel.total_msat ? Math.floor(channel.total_msat / 1000) : 0;
				const local = channel.to_us_msat ? Math.floor(channel.to_us_msat / 1000) : 0;
				return total - local;
			}
			case 'eclair':
				return Math.floor(
					(channel.data?.commitments?.active[0]?.localCommit?.spec?.toRemote || 0) / 1000
				);
			default:
				return 0;
		}
	}

	private isChannelActive(type: string, channel: any): boolean {
		switch (type) {
			case 'lnd':
				return channel.active;
			case 'coreln':
				return channel.state === 'CHANNELD_NORMAL';
			case 'eclair':
				return channel.state === 'NORMAL';
			default:
				return false;
		}
	}
}
