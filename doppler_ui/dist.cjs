const { bin } = require('./package.json');
const execSync = require('child_process').execSync;
const path = require('path');

// Compute the target we're building for
const bunTargets = {
	'x86_64-pc-windows-msvc': 'bun-windows-x64',
	'aarch64-apple-darwin': 'bun-darwin-arm64',
	'x86_64-apple-darwin': 'bun-darwin-x64',
	'aarch64-unknown-linux-gnu': 'bun-linux-arm64',
	'x86_64-unknown-linux-gnu': 'bun-linux-x64'
};
const distTarget = process.env.CARGO_DIST_TARGET || process.env.DIST_TARGET;
if (!distTarget) {
	throw "DIST_TARGET isn't set, so we don't know what platform to build!";
}
const bunTarget = bunTargets[distTarget];
if (!bunTarget) {
	throw `To the the best of our knowledge, bun does not support building for ${distTarget}`;
}
const binExt = distTarget.includes('windows') ? '.exe' : '';

// Check if the target is Darwin (macOS)
const isDarwin = distTarget.includes('apple-darwin');
if (isDarwin) {
	console.log('Darwin detected. Installing @rollup/rollup-darwin-x64...');
	execSync('npm install @rollup/rollup-darwin-x64', { stdio: 'inherit' });
	console.log('Removing old installed dependencies ...');
	execSync('rm package-lock.json', { stdio: 'inherit' });
	execSync('rm -rf node_modules', { stdio: 'inherit' });
}

// Setup npm

// Setup npm
console.log('Installing dependencies with npm...');
execSync('npm install', { stdio: 'inherit' });

// Run npm build
console.log('Running npm build...');
execSync('npm run build', { stdio: 'inherit' });

// Setup bun
console.log('Installing dependencies with bun...');
execSync('bun install', { stdio: 'inherit' });

// for each binary, run bun
for (binName of Object.keys(bin)) {
	const binScript = bin[binName];
	const binPath = `${binName}${binExt}`;

	// Change to the directory of the script
	const scriptDir = path.dirname(binScript);
	process.chdir(scriptDir);

	// Get the relative path of the script from the new working directory
	const relativeScriptPath = path.basename(binScript);

	// Run bun build with the relative path
	execSync(
		`bun build ${relativeScriptPath} --compile --target ${bunTarget} --outfile ${path.join(
			'..',
			binPath
		)}`,
		{
			stdio: 'inherit'
		}
	);
}
