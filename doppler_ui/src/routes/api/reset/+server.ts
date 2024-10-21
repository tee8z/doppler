import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import path from 'path';
import { spawn } from 'child_process';
import { parse } from 'ini';
import { resolve } from 'path';
import { v7 } from 'uuid';

// Read and parse the INI config file
const configPath = path.join(process.cwd(), 'ui_config/server.conf.ini');
const config = parse(fs.readFileSync(configPath, 'utf-8'));

const LOGS_FOLDER = path.join(process.cwd(), config.paths.logsFolder);

interface ResetPayload {
	id?: string;
}

function deleteFile(filePath: string): Promise<void> {
	return new Promise((resolve, reject) => {
		const fullPath = path.resolve(filePath);
		fs.unlink(fullPath, (err) => {
			if (err) {
				if (err.code === 'ENOENT') {
					console.log(`File not found, skipping deletion: ${filePath}`);
					resolve();
				} else {
					console.error(`Error deleting file ${filePath}:`, err);
					reject(err);
				}
			} else {
				console.log(`Successfully deleted file: ${fullPath}`);
				resolve();
			}
		});
	});
}

export const POST: RequestHandler = async function (event) {
	try {
		const body: ResetPayload = await event.request.json();
		let logFilename: string;
		let operationId: string;

		if (body.id) {
			operationId = body.id;
			logFilename = path.join(LOGS_FOLDER, `${operationId}.log`);
		} else {
			operationId = v7();
			logFilename = path.join(LOGS_FOLDER, `reset_${operationId}.log`);
		}
		await deleteFile('ui_config/info.conf.ini');

		const resetScript = `${config.paths.scriptsFolder}/reset.sh`;
		console.log(resetScript);
		const process = spawn(resetScript, {
			cwd: config.paths.currentWorkingDirectory,
			detached: true,
			stdio: ['ignore', 'pipe', 'pipe']
		});

		if (!process.pid) {
			console.error('Failed to spawn reset process');
			return json({ error: 'Failed to spawn reset process' }, { status: 500 });
		}

		const logStream = fs.createWriteStream(logFilename, { flags: 'a' });
		process.unref();

		// Use 'data' events instead of pipe
		process.stdout.on('data', (data) => {
			logStream.write(data);
		});
		process.stderr.on('data', (data) => {
			logStream.write(data);
		});

		process.on('error', (error) => {
			const errorMessage = `Error spawning process: ${error.message}\n`;
			console.error(errorMessage);
			logStream.write(errorMessage);
		});

		// Handle process completion
		process.on('close', (code) => {
			const closeMessage = `Child process exited with code ${code}\n`;
			console.log(closeMessage);
			logStream.write(closeMessage, () => {
				// End the stream only after ensuring all data has been written
				logStream.end();
			});
		});

		return json({
			message: 'Reset script execution started in the background',
			logPath: logFilename
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
