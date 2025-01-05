import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import { parse } from 'ini';
import { resolve } from 'path';
import * as path from 'path';
import { UI_CONFIG_PATH } from '$env/static/private';

export interface ConnectionConfig {
	macaroon: string;
	rune: string;
	user: String;
	password: string;
	type: string;
	host: string;
	rpc_port: string;
	p2p_port: string;
}

export interface Connections {
	[key: string]: ConnectionConfig;
}

function safeReadFileSync(path: string): Buffer | null {
	try {
		return fs.readFileSync(path);
	} catch (error) {
		if (error.code === 'ENOENT') {
			console.warn(`File not found: ${path}`);
			return null;
		}
		// For other types of errors, you might want to rethrow
		throw error;
	}
}

const configPath = UI_CONFIG_PATH || process.env.UI_CONFIG_PATH || path.join(process.cwd(), '/build/ui_config');

//TODO: have the info.conf be change based on the run script
export const GET: RequestHandler = async function () {
	let parsedConfig;
	try {
		const filePath = resolve(`${configPath}/info.conf.ini`);
		const fileContent = fs.readFileSync(filePath, 'utf8');
		parsedConfig = parse(fileContent);
	} catch (error) {
		// If the file doesn't exist or there's any other error, return an empty object
		return json({});
	}
	try {
		const connections: Connections = {};
		for (const section in parsedConfig) {
			const sectionConfig = parsedConfig[section];
			if (sectionConfig.TYPE == 'lnd') {
				const macaroon = safeReadFileSync(sectionConfig.ADMIN_MACAROON_PATH);
				if (macaroon === null) {
					continue;
				}
				const readMacaroon = macaroon.toString('hex');
				connections[section] = {
					macaroon: readMacaroon,
					host: sectionConfig.API_ENDPOINT,
					type: sectionConfig.TYPE,
					rune: '',
					password: '',
					user: '',
					rpc_port: '',
					p2p_port: ''
				};
			} else if (sectionConfig.TYPE === 'coreln') {
				connections[section] = {
					macaroon: '',
					rune: sectionConfig.RUNE,
					host: sectionConfig.API_ENDPOINT,
					type: sectionConfig.TYPE,
					password: '',
					user: '',
					rpc_port: '',
					p2p_port: ''
				};
			} else if (sectionConfig.TYPE === 'eclair') {
				connections[section] = {
					macaroon: '',
					rune: '',
					host: sectionConfig.API_ENDPOINT,
					password: sectionConfig.API_PASSWORD,
					type: sectionConfig.TYPE,
					user: '',
					rpc_port: '',
					p2p_port: ''
				};
			} else if (sectionConfig.TYPE === 'bitcoind') {
				connections[section] = {
					macaroon: '',
					rune: '',
					host: '',
					password: sectionConfig.PASSWORD,
					type: sectionConfig.TYPE,
					user: sectionConfig.USER,
					rpc_port: sectionConfig.RPC,
					p2p_port: sectionConfig.P2P
				};
			} else if (sectionConfig.TYPE === 'esplora') {
				connections[section] = {
					macaroon: '',
					rune: '',
					host: sectionConfig.API_ENDPOINT,
					password: '',
					type: sectionConfig.TYPE,
					user: '',
					rpc_port: sectionConfig.ELECTRUM_PORT,
					p2p_port: ''
				};
			} else {
				throw Error(`node type ${sectionConfig.TYPE} not supported yet!`);
			}
		}
		return json({
			...connections
		});
	} catch (error) {
		console.warn(error);
		return json({
			error: 'Failed to read connections directory in docker volumes'
		});
	}
};
