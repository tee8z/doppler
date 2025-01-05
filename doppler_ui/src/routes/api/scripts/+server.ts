import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';
import { parse } from 'ini';
import { getDirectoryTree } from '$lib/file_accessor';
import { UI_CONFIG_PATH } from '$env/static/private';

const configPath = UI_CONFIG_PATH || process.env.UI_CONFIG_PATH || path.join(process.cwd(), '/build/ui_config');
const config = parse(fs.readFileSync(`${configPath}/server.conf.ini`, 'utf-8'));

const DOPPLER_SCRIPTS_FOLDER = config.paths.dopplerScriptsFolder;

export const GET: RequestHandler = () => {
	try {
		const tree = getDirectoryTree(DOPPLER_SCRIPTS_FOLDER);
		return json(tree);
	} catch (error) {
		return new Response(JSON.stringify({ error: 'Failed to get folder tree' }), {
			status: 500,
			headers: {
				'Content-Type': 'application/json'
			}
		});
	}
};
