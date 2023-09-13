/* eslint-disable @typescript-eslint/no-explicit-any */
import { Order, javascriptGenerator } from 'blockly/javascript';
export function initGenerators() {
	javascriptGenerator.forBlock['bitcoind_miner'] = function (
		block: any,
		generator: { valueToCode: (arg0: any, arg1: string, arg2: Order) => any }
	) {
		const name = generator.valueToCode(block, 'NAME', Order.ATOMIC);
		const blocktime = generator.valueToCode(block, 'BlockTime', Order.ATOMIC);
		return `BITCOIND_MINER ${name.replace(/'/g, '')} ${blocktime && blocktime + 's'}\n`;
	};

	javascriptGenerator.forBlock['ln_pair'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const node_type = block.getFieldValue('NODE_TYPE');
		const node_name = block.getFieldValue('NODE_NAME');
		const bitcoind_name = block.getFieldValue('BITCOIND_NAME');
		return `${node_type} ${node_name} PAIR ${bitcoind_name}\n`;
	};

	javascriptGenerator.forBlock['visualizer'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const name = block.getFieldValue('CMD');
		return `VISUALIZER ${name.replace(/'/g, '')}\n`;
	};

	javascriptGenerator.forBlock['up'] = function () {
		return `UP\n`;
	};

	javascriptGenerator.forBlock['open_channel'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const node_name = block.getFieldValue('NODE_NAME');
		const channel_partner = block.getFieldValue('CHANNEL_PARTNER');
		const channel_size = block.getFieldValue('CHANNEL_SIZE');
		return `${node_name} OPEN_CHANNEL ${channel_partner} ${Number(channel_size)}\n`;
	};

	javascriptGenerator.forBlock['mine_blocks'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const bitcoind_name = block.getFieldValue('BITCOIND_NAME');
		const name = block.getFieldValue('NAME');
		return `${bitcoind_name} MINE_BLOCKS ${Number(name)}\n`;
	};

	javascriptGenerator.forBlock['skip_conf'] = function () {
		return `SKIP_CONF\n`;
	};

	javascriptGenerator.forBlock['close_channel'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const node_name = block.getFieldValue('NODE_NAME');
		const channel_partner = block.getFieldValue('CHANNEL_PARTNER');
		return `${node_name} CLOSE_CHANNEL ${channel_partner}\n`;
	};

	javascriptGenerator.forBlock['send_ln'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const node_name = block.getFieldValue('NODE_NAME');
		const channel_partner = block.getFieldValue('CHANNEL_PARTNER');
		const amt = block.getFieldValue('AMT');
		return `${node_name} SEND_LN ${channel_partner} ${Number(amt)}\n`;
	};

	javascriptGenerator.forBlock['bitcoind'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const bitcoind_name = block.getFieldValue('BITCOIND_NAME');
		return `BITCOIND ${bitcoind_name}\n`;
	};

	javascriptGenerator.forBlock['send_on_chain'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const node_name = block.getFieldValue('NODE_NAME');
		const peer = block.getFieldValue('PEER');
		const amt = block.getFieldValue('AMT');
		return `${node_name} SEND_ON_CHAIN ${peer} AMT ${Number(amt)}\n`;
	};

	javascriptGenerator.forBlock['loop'] = function (
		block: { getFieldValue: (arg0: string) => any },
		generator: { statementToCode: (arg0: any, arg1: string, arg2: Order) => any }
	) {
		const times = block.getFieldValue('TIMES');
		return `LOOP ${Number(times)}\n${generator.statementToCode(block, 'NAME', Order.NONE)}END\n`;
	};

	javascriptGenerator.forBlock['loop_every'] = function (
		block: { getFieldValue: (arg0: string) => any },
		generator: { statementToCode: (arg0: any, arg1: string, arg2: Order) => any }
	) {
		const times = block.getFieldValue('TIMES');
		return `LOOP_EVERY ${Number(times)}\n${generator.statementToCode(
			block,
			'NAME',
			Order.NONE
		)}END\n`;
	};

	javascriptGenerator.forBlock['bitcoind_miner'] = function (block: {
		getFieldValue: (arg0: string) => any;
	}) {
		const name = block.getFieldValue('BITCOIND_NAME');
		const blocktime = block.getFieldValue('BLOCK_TIME');
		return `BITCOIND_MINER ${name} ${Number(blocktime)}s\n`;
	};
}
