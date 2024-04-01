import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import { parse } from 'ini';
import path from 'path';
import { resolve } from 'path';

export interface ConnectionConfig {
    macaroon: string;
    tls: string;
    host: string;
}

export interface Connections {
    [key: string]: ConnectionConfig;
}

export const GET: RequestHandler = async function () {
    try {
        const filePath = resolve('config/info.conf');
        const fileContent = fs.readFileSync(filePath, 'utf8');
        const parsedConfig = parse(fileContent);
        const connections: Connections = {};
        for (const section in parsedConfig) {
            const sectionConfig = parsedConfig[section];
            const readMacaroon = fs.readFileSync(sectionConfig.ADMIN_MACAROON_PATH).toString('hex');
            const readTls = fs.readFileSync(sectionConfig.TLS_CERT_PATH, 'utf-8');
            connections[section] = {
                macaroon: readMacaroon,
                tls: readTls,
                host: sectionConfig.API_ENDPOINT
            };
        }
        return json({
            ...connections
        });
    } catch (error) {
        console.error(error);
        return json({
            error: 'Failed to read connections directory'
        });
    }
};