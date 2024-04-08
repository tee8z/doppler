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
        let requestOptions: any = { method: 'GET', headers: fetchHeaders, credentials: "include" };
        if (target.includes("https")) {
            const httpsAgent = new https.Agent({
                rejectUnauthorized: false,
            });
            requestOptions['agent'] = httpsAgent;
        }
        const response = await fetch(target, requestOptions);
        if (!response.ok) {
            throw new Error(`Failed to fetch from node: ${await response.text()}`)
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
        let requestOptions: any = { method: 'POST', headers: fetchHeaders, credentials: "include" };
        if (target.includes("https")) {
            const httpsAgent = new https.Agent({
                rejectUnauthorized: false,
            });
            requestOptions['agent'] = httpsAgent;
        }
        const bodyText = await event.request.text();
        if (bodyText) {
            const body = JSON.parse(bodyText);
            requestOptions['body'] = JSON.stringify(body);
        }
        const response = await fetch(target, requestOptions);
        if (!response.ok) {
            throw new Error(`Failed to fetch from node: ${await response.text()}`)
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