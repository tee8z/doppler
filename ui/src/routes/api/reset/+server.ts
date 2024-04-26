import { json, type RequestHandler } from '@sveltejs/kit';
import { exec } from 'node:child_process';
import { fileURLToPath } from 'url';
import path from 'path';
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const workingDirectory = path.resolve(__dirname, '../../../../..', '.');
console.log(`Working directory: ${workingDirectory}`); // Log the working directory

const scriptPath = './scripts/reset.sh';

export const POST: RequestHandler = async function (event) {
    exec(`bash ${scriptPath}`, { cwd: workingDirectory }, (err, stdout, stderr) => {
        if (err) {
            console.error(`exec error: ${err}`);
            return;
        }
        console.log(`stdout: ${stdout}`);
        console.error(`stderr: ${stderr}`);
    });

    return json({ message: "Bash script executed" });
};