import { BaseRequestHandler, type NodeRequests } from "./nodes";

export interface LndRequests {
    new(base_url: string, macaroon: string): void;
}

export class LndRequests implements LndRequests, NodeRequests {
    requestHandler: BaseRequestHandler

    constructor(base_url: string, macaroon: string) {
        const header = {
            'Grpc-Metadata-macaroon': macaroon,
        };
        const proxy = '/api/proxy';
        this.requestHandler = new BaseRequestHandler(base_url, header, proxy);
    }
    //API docs: https://lightning.engineering/api-docs/api/lnd/index.html

    async fetchChannels(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/channels`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchInfo(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/getinfo`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchBalance(): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/balance/blockchain`
        return await this.requestHandler.send_request(url, "GET", false);
    }

    async fetchSpecificNodeInfo(pubkey: String): Promise<any> {
        let url = `${this.requestHandler.base_url}/v1/graph/node/${pubkey}`
        return await this.requestHandler.send_request(url, "GET", false);
    }
};