export interface ConnectionConfig {
    macaroon: string;
    password: string;
    host: string;
    type: string;
}

export interface Connections {
    [key: string]: ConnectionConfig;
}

export async function getConnections(): Promise<Connections> {
    try {
        const response = await fetch('/api/connections');
        if (!response.ok) {
            throw new Error('Failed to fetch connections');
        }
        const connections = await response.json();
        return connections
    } catch (error) {
        console.error('Error fetching connections:', error);
    }
    return {}
}