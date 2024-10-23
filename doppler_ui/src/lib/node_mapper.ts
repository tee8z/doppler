interface Node {
	id: string;
	alias: string;
	known: string;
	type?: string;
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

	private addNode(nodeId: string, alias: string, type?: string) {
		if (!this.nodeMap.has(nodeId)) {
			// First try to find a known connection
			const known = this.nodeConnections.find((node) => node.pubkey === nodeId);

			const nodeInfo: Node = {
				id: nodeId,
				alias: known?.alias || alias, // Use known alias if available, otherwise use provided alias
				known: nodeId
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

			this.addNode(nodeId, key, value.type);
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

	private getChannelPoint(type: string, channel: any): string | null {
		switch (type) {
			case 'lnd':
				return channel.channel_point;
			case 'coreln': {
				// coreln channels have either channel_point or funding_txid + funding_output
				if (channel.channel_point) {
					return channel.channel_point;
				}
				if (channel.funding?.funding_txid || channel.funding_txid) {
					const txid = channel.funding?.funding_txid || channel.funding_txid;
					const output = channel.funding?.funding_output ?? channel.funding_output;
					if (txid && output !== undefined) {
						return `${txid}:${output}`;
					}
				}
				// Try getting from fundingTx if available
				if (channel.funding_txid) {
					return `${channel.funding_txid}:${channel.funding_output || 0}`;
				}
				return null;
			}
			case 'eclair': {
				const outPoint = channel.data?.commitments?.active[0]?.fundingTx?.outPoint;
				return outPoint || null;
			}
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

	private getChannelPoint(type: string, channel: any): string | null {
		switch (type) {
			case 'lnd':
				return channel.channel_point;
			case 'coreln': {
				// If we have funding_txid and funding_outnum, use those
				if (channel.funding_txid && channel.funding_outnum !== undefined) {
					return `${channel.funding_txid}:${channel.funding_outnum}`;
				}
				// Fallback to other potential formats
				if (channel.channel_point) {
					return channel.channel_point;
				}
				return null;
			}
			case 'eclair': {
				// Get outPoint from active commitments if available
				const outPoint = channel.data?.commitments?.active[0]?.fundingTx?.outPoint;
				if (outPoint) {
					return outPoint;
				}
				// If no outPoint, try to construct from channel ID
				// Note: This is a fallback that might need adjustment based on your needs
				return null;
			}
			default:
				return null;
		}
	}

	private getChannelId(type: string, channel: any): string | null {
		// First try to get the consistent channel point
		const channelPoint = this.getChannelPoint(type, channel);
		if (channelPoint) {
			return channelPoint;
		}

		// If we can't get channel point, fall back to channel_id
		// but we need to ensure it's consistent across implementations
		switch (type) {
			case 'eclair':
				return channel.data?.commitments?.params?.channelId || null;
			case 'coreln':
				return channel.channel_id || null;
			default:
				return null;
		}
	}

	private mapChannels(type: string, nodeKey: string, nodeData: any) {
		const currentPubkey = this.getNodeId(nodeData);
		if (!currentPubkey) return;
		const channels = this.getChannels(type, nodeData);
		if (!channels) return;

		channels.forEach((channel: any) => {
			// Get both channel point and channel ID
			const channelPoint = this.getChannelPoint(type, channel);
			const channelId = this.getChannelId(type, channel);

			// If we can't identify the channel through either method, skip it
			if (!channelPoint && !channelId) return;

			const remotePubkey = this.getRemotePubkey(type, channel);
			if (!remotePubkey) return;

			// Create a consistent identifier that works across implementations
			const sortedPubkeys = [currentPubkey, remotePubkey].sort();
			const consistentChannelId = channelPoint
				? `${channelPoint}:${sortedPubkeys[0]}:${sortedPubkeys[1]}`
				: `${channelId}:${sortedPubkeys[0]}:${sortedPubkeys[1]}`;

			// Add remote node if we haven't seen it before
			this.addNode(remotePubkey, remotePubkey);

			// Get alias
			const currentAlias = this.getNodeAlias(currentPubkey);

			// Determine channel properties
			const isInitiator = this.isChannelInitiator(type, channel);
			const isActive = this.isChannelActive(type, channel);

			// Create data entry for this node's view of the channel
			const channelData: ChannelData = {
				type,
				alias: currentAlias,
				channel,
				perspective: isInitiator ? 'source' : 'target'
			};

			if (!this.channelMap.has(consistentChannelId)) {
				// Creating new channel
				const channelInfo: Channel = {
					source: isInitiator ? currentPubkey : remotePubkey,
					target: isInitiator ? remotePubkey : currentPubkey,
					channel_id: channelPoint || channelId || '', // Prefer channel point if available
					capacity: this.getChannelCapacity(type, channel),
					local_balance: this.getLocalBalance(type, channel),
					remote_balance: this.getRemoteBalance(type, channel),
					initiator: isInitiator,
					active: isActive,
					data: [channelData]
				};
				this.channelMap.set(consistentChannelId, channelInfo);
				this.edges.push(channelInfo);
				this.uniqueChannels.add(consistentChannelId);
			} else {
				// Channel exists, update with this node's perspective
				const existingChannel = this.channelMap.get(consistentChannelId)!;

				// Check if we already have data from this node
				const hasNodeData = existingChannel.data.some(
					(d) => d.type === type && d.alias === currentAlias
				);

				// Only add if we don't have this node's view yet
				if (!hasNodeData) {
					existingChannel.data.push(channelData);
				}

				// Merge channel properties
				existingChannel.active = existingChannel.active && isActive;

				// Update balances from initiator's perspective
				if (isInitiator) {
					existingChannel.capacity = this.getChannelCapacity(type, channel);
					existingChannel.local_balance = this.getLocalBalance(type, channel);
					existingChannel.remote_balance = this.getRemoteBalance(type, channel);
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
				return channel.our_amount_msat ? Math.floor(channel.our_amount_msat / 1000) : 0;
			case 'eclair':
				return Math.floor(
					(channel.data?.commitments?.active[0]?.localCommit?.spec?.toLocal || 0) / 1000
				);
			default:
				return 0;
		}
	}

	private getRemoteBalance(type: string, channel: any): number {
		switch (type) {
			case 'lnd':
				return parseInt(channel.remote_balance) || 0;
			case 'coreln': {
				const total = channel.amount_msat ? Math.floor(channel.amount_msat / 1000) : 0;
				const local = channel.our_amount_msat ? Math.floor(channel.our_amount_msat / 1000) : 0;
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
