import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import { parse } from 'ini';
import { resolve } from 'path';
import * as path from 'path';

export interface ConnectionConfig {
	macaroon: string;
	password: string;
	type: string;
	host: string;
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

const configPath = process.env.UI_CONFIG_PATH || path.join(process.cwd(), '/build/ui_config');

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
					password: ''
				};
			} else if (sectionConfig.TYPE === 'coreln') {
				const macaroonBuffer = safeReadFileSync(sectionConfig.ACCESS_MACAROON_PATH);
				if (macaroonBuffer === null) {
					continue;
				}
				const readMacaroon = Buffer.from(macaroonBuffer).toString('hex');
				connections[section] = {
					macaroon: readMacaroon,
					host: sectionConfig.API_ENDPOINT,
					type: sectionConfig.TYPE,
					password: ''
				};
			} else if (sectionConfig.TYPE === 'eclair') {
				connections[section] = {
					macaroon: '',
					host: sectionConfig.API_ENDPOINT,
					password: sectionConfig.API_PASSWORD,
					type: sectionConfig.TYPE
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
