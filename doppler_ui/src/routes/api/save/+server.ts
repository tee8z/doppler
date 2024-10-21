import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';
import { parse } from 'ini';

const configPath = path.join(process.cwd(), 'ui_config/server.conf.ini');
const config = parse(fs.readFileSync(configPath, 'utf-8'));

const DOPPLER_SCRIPTS_FOLDER = path.join(process.cwd(), config.paths.dopplerScriptsFolder);
const LOGS_FOLDER = path.join(process.cwd(), config.paths.logsFolder);

interface ScriptPayload {
	id: string;
	fullPath: string;
	script: string;
}

//TODO: pull script from save folder instead of saving it here and running it
export const POST: RequestHandler = async function (event: any) {
	try {
		const body: ScriptPayload = await event.request.json();
		const { id, fullPath, script } = body;

		if (!id || !fullPath || !script) {
			return json(
				{ error: 'Invalid input. An id, fullPath, and script are required.' },
				{ status: 400 }
			);
		}

		[DOPPLER_SCRIPTS_FOLDER, LOGS_FOLDER].forEach((folder) => {
			if (!fs.existsSync(folder)) {
				fs.mkdirSync(folder, { recursive: true });
			}
		});

		const localPath = path.join(DOPPLER_SCRIPTS_FOLDER, fullPath);
		const dir = path.dirname(localPath);

		// Create directories if they don't exist
		fs.mkdirSync(dir, { recursive: true });

		// Save the script
		fs.writeFileSync(localPath, script);

		return json({
			message: 'Script saved',
			scriptPath: fullPath
		});
	} catch (error) {
		console.error('Error:', error);
		return json(
			{
				error: 'An error occurred while processing the request'
			},
			{ status: 500 }
		);
	}
};
