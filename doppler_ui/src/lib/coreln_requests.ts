import { BaseRequestHandler, type NodeRequests } from "./nodes";

export interface CorelnRequests {
    new(base_url: string, macaroon: string): void;
}

export class CorelnRequests implements CorelnRequests, NodeRequests {
    requestHandler: BaseRequestHandler

    constructor(base_url: string, macaroon: string) {
        const header = {
            'macaroon': macaroon,
            'encodingtype': 'hex'
        };
        const proxy = '/api/proxy';
        this.requestHandler = new BaseRequestHandler(base_url, header, proxy);
    }
    //API docs: https://github.com/Ride-The-Lightning/c-lightning-REST

    async fetchChannels(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/channel/listChannels`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchInfo(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/getinfo`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchBalance(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/getBalance`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/network/listNode/${pubkey}`
        return await this.requestHandler.send_request(url, "GET", false);
    }
};