import { json, type RequestHandler } from '@sveltejs/kit';
import https from 'https';
import fetch from 'node-fetch';

export const GET: RequestHandler = async function (event) {
    try {
        const target = event.request.headers.get('target');
        const fetchHeaders = new Headers();
        for (const [key, value] of event.request.headers) {
            if (key != 'target') {
                fetchHeaders.append(key, value);
            }
        }
        if (!target) {
            console.error("Failed to proxy request as target is missing", event);
            return json({
                error: "Failed, 'target' header to proxy GET request is required"
            })
        }
        const httpsAgent = new https.Agent({
            rejectUnauthorized: false,
        });
        const response = await fetch(target, { method: 'GET', agent: httpsAgent, headers: fetchHeaders });
        if (!response.ok) {
            throw new Error("Failed to fetch from node")
        }
        const payload = await response.json();
        return json(payload);
    } catch (error) {
        console.error(error);
        return json({
            error: 'Failed to proxy response to nodes'
        });
    }
};

export const POST: RequestHandler = async function (event) {
    try {
        const target = event.request.headers.get('target');
        const fetchHeaders = new Headers();
        for (const [key, value] of event.request.headers) {
            if (key != 'target') {
                fetchHeaders.append(key, value);
            }
        }
        if (!target) {
            console.error("Failed to proxy request as target is missing", event);
            return json({
                error: "Failed, 'target' header to proxy POST request is required"
            })
        }
        const httpsAgent = new https.Agent({
            rejectUnauthorized: false,
        });
        const response = await fetch(target, { method: 'POST', agent: httpsAgent, headers: fetchHeaders, body: await event.request.json() });
        if (!response.ok) {
            throw new Error("Failed to fetch from node")
        }
        const payload = await response.json();
        return json(payload);
    } catch (error) {
        console.error(error);
        return json({
            error: 'Failed to proxy response to nodes'
        });
    }
};