import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import fs from 'fs';
import path from 'path';
import { parse } from 'ini';

const configPath = path.join(process.cwd(), 'ui_config/server.conf.ini');
const config = parse(fs.readFileSync(configPath, 'utf-8'));

const DOPPLER_SCRIPTS_FOLDER = path.join(process.cwd(), config.paths.dopplerScriptsFolder);

export const GET: RequestHandler = function (event: any) {
	console.log('Received GET request for script content');
	const scriptPath = event.url.searchParams.get('scriptPath');

	if (!scriptPath) {
		console.error('scriptPath parameter is missing');
		return json({ error: 'scriptPath parameter is required' }, { status: 400 });
	}

	try {
		// Ensure the path is safe and within your allowed directory
		const safePath = path.normalize(scriptPath).replace(/^(\.\.(\/|\\|$))+/, '');
		const fullPath = path.join(DOPPLER_SCRIPTS_FOLDER, safePath);

		// Check if file exists
		if (!fs.existsSync(fullPath)) {
			throw new Error('File not found');
		}

		// Read the file synchronously as utf-8 text
		const fileContent = fs.readFileSync(fullPath, 'utf-8');

		// Set appropriate headers for text content
		const headers = new Headers();
		headers.append('Content-Type', 'text/plain; charset=utf-8');

		// Return the file content as a text Response
		return new Response(fileContent, {
			status: 200,
			headers: headers
		});
	} catch (error) {
		console.error('Error reading file:', error);
		return json({ error: 'File not found or error reading file' }, { status: 404 });
	}
};
