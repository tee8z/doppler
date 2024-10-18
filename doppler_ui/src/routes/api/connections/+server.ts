import { json, type RequestHandler } from '@sveltejs/kit';
import fs from 'fs';
import { parse } from 'ini';
import { resolve } from 'path';

export interface ConnectionConfig {
    macaroon: string;
    password: string;
    type: string;
    host: string;
}

export interface Connections {
    [key: string]: ConnectionConfig;
}

export const GET: RequestHandler = async function () {
    try {
        const filePath = resolve('ui_config/info.conf');
        const fileContent = fs.readFileSync(filePath, 'utf8');
        const parsedConfig = parse(fileContent);
        const connections: Connections = {};
        for (const section in parsedConfig) {
            const sectionConfig = parsedConfig[section];
            if (sectionConfig.TYPE == 'lnd') {
                const readMacaroon = fs.readFileSync(sectionConfig.ADMIN_MACAROON_PATH).toString('hex');
                connections[section] = {
                    macaroon: readMacaroon,
                    host: sectionConfig.API_ENDPOINT,
                    type: sectionConfig.TYPE,
                    password: ''
                };
            } else if (sectionConfig.TYPE === 'coreln') {
                const macaroonBuffer = fs.readFileSync(sectionConfig.ACCESS_MACAROON_PATH);
                const readMacaroon = Buffer.from(macaroonBuffer).toString("hex");
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
                throw Error(`node type ${sectionConfig.TYPE} not supported yet!`)
            }
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