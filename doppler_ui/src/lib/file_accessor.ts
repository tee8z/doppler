import fs from 'fs';
import path from 'path';

export function getDirectoryTree(dirPath: string): any {
	try {
		const stats = fs.statSync(dirPath);

		if (!stats.isDirectory()) {
			return { label: path.basename(dirPath) };
		}

		const children = fs
			.readdirSync(dirPath)
			.map((child: string) => {
				try {
					return getDirectoryTree(path.join(dirPath, child));
				} catch (error) {
					console.error(`Error accessing ${child}: ${error}`);
					return null;
				}
			})
			.filter((child: any) => child !== null);

		return {
			label: path.basename(dirPath),
			children: children.length > 0 ? children : undefined
		};
	} catch (error) {
		if (error.code === 'ENOENT') {
			console.warn(`Directory not found: ${dirPath}`);
			return {
				label: path.basename(dirPath),
				children: []
			};
		}
		throw error;
	}
}
