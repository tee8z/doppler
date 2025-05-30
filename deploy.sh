#!/bin/bash

set -e

# Display welcome banner
echo "======================================================="
echo "   Doppler - Node Cluster Tool Setup with CORS Support"
echo "======================================================="

# Automatically detect latest Doppler version from GitHub
echo "Detecting latest Doppler release..."
LATEST_VERSION=$(curl -s https://api.github.com/repos/tee8z/doppler/releases/latest | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
  echo "Could not detect latest version. Using default version."
  DOPPLER_VERSION="${1:-v0.4.0}"  # Default to v0.4.0 if not provided and couldn't detect latest
else
  echo "Latest version detected: $LATEST_VERSION"
  DOPPLER_VERSION="${1:-$LATEST_VERSION}"  # Use latest version if not provided
fi

# Admin credentials can be parameterized
ADMIN_USER="${2:-admin}"
ADMIN_PASS="${3:-$(openssl rand -base64 12)}"

echo "Installing Doppler $DOPPLER_VERSION..."

# Install dependencies
echo "Installing dependencies..."
apt-get update
apt-get install -y curl tar haproxy

# Install Docker
echo "Installing Docker..."
apt-get install -y docker.io

# Start and enable Docker
systemctl start docker
systemctl enable docker

# Install Docker Compose V2
echo "Installing Docker Compose V2..."
mkdir -p /usr/local/lib/docker/cli-plugins
curl -SL "https://github.com/docker/compose/releases/download/v2.24.6/docker-compose-linux-x86_64" -o /usr/local/lib/docker/cli-plugins/docker-compose
chmod +x /usr/local/lib/docker/cli-plugins/docker-compose

# Install NVM
echo "Setting up Node.js environment..."
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.3/install.sh | bash
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"
nvm install --lts

# Download and run the Doppler installer script
echo "Installing Doppler..."
curl -sSL https://raw.githubusercontent.com/tee8z/doppler/master/doppler-installer.sh | bash -s "$DOPPLER_VERSION"

# Set up HAProxy for UI authentication
echo "Configuring HAProxy for UI authentication..."
cat > /etc/haproxy/haproxy.cfg << EOF
global
    log /dev/log local0
    log /dev/log local1 notice
    maxconn 4096
    user haproxy
    group haproxy
    daemon

defaults
    log     global
    mode    http
    option  httplog
    option  dontlognull
    timeout connect 5000
    timeout client  50000
    timeout server  50000

# User list for basic authentication
userlist doppler_users
    user ${ADMIN_USER} insecure-password ${ADMIN_PASS}

# Frontend for the main UI on port 80 (with basic auth)
frontend doppler_ui
    bind *:80

    # Basic authentication configuration
    acl auth_ok http_auth(doppler_users)
    http-request auth realm Doppler-Admin-Area if !auth_ok

    # Forward to backend UI service
    default_backend doppler_ui_backend

# UI Backend
backend doppler_ui_backend
    server ui_server 127.0.0.1:3000
EOF

# Create a CORS proxy for service ports (9000-11000)
echo "Setting up CORS proxy for service ports..."
mkdir -p /opt/cors-proxy
cat > /opt/cors-proxy/package.json << EOF
{
  "name": "cors-proxy",
  "version": "1.0.0",
  "description": "CORS proxy for Doppler services",
  "main": "server.js",
  "dependencies": {
    "http-proxy": "^1.18.1"
  }
}
EOF

# Create the CORS proxy server script
cat > /opt/cors-proxy/server.js << 'EOF'
const http = require("http");
const httpProxy = require("http-proxy");

// Define port range
const START_PORT = 9090;
const END_PORT = 10010;
const PORT_OFFSET = 1000; // External ports will be internal + 1000

console.log(`Starting CORS proxy for ports ${START_PORT}-${END_PORT}`);

// Process ports in sequence
const setupProxy = (index) => {
  const internalPort = START_PORT + index;
  
  if (internalPort > END_PORT) {
    console.log("All CORS proxies are now running");
    return;
  }
  
  const externalPort = internalPort + PORT_OFFSET;
  
  try {
    // Create both HTTP and HTTPS proxies for each port
    const httpProxyServer = httpProxy.createProxyServer({
      target: `http://localhost:${internalPort}`,
      ws: true,
      secure: false,
      changeOrigin: true
    });

    const httpsProxyServer = httpProxy.createProxyServer({
      target: `https://localhost:${internalPort}`,
      ws: true,
      secure: false,
      changeOrigin: true
    });

    const setupProxyEvents = (proxy) => {
      proxy.on("error", (err, req, res) => {
        // Only log errors if not handled elsewhere
        if (res && !res.headersSent && !req._handledError) {
          req._handledError = true;
          console.error(`Proxy error (${externalPort}->${internalPort}):`, err.message);
        }
      });

      proxy.on("proxyRes", function (proxyRes, req, res) {
        // Add CORS headers
        proxyRes.headers["Access-Control-Allow-Origin"] = "*";
        proxyRes.headers["Access-Control-Allow-Methods"] = "GET, POST, OPTIONS, PUT, DELETE, PATCH";
        proxyRes.headers["Access-Control-Allow-Headers"] = "X-Requested-With, Content-Type, Authorization, Grpc-Metadata-macaroon, Rune";
        proxyRes.headers["Access-Control-Expose-Headers"] = "*";
      });
    };

    setupProxyEvents(httpProxyServer);
    setupProxyEvents(httpsProxyServer);

    // Create handler for requests
    const requestHandler = (req, res) => {
      // Handle preflight OPTIONS requests directly
      if (req.method === "OPTIONS") {
        res.writeHead(200, {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS, PUT, DELETE, PATCH",
          "Access-Control-Allow-Headers": "X-Requested-With, Content-Type, Authorization, Grpc-Metadata-macaroon, Rune",
          "Access-Control-Expose-Headers": "*"
        });
        res.end();
        return;
      }

      // Try HTTPS first, fallback to HTTP
      httpsProxyServer.web(req, res, {}, (err) => {
        if (err) {
          // If HTTPS fails, try HTTP
          httpProxyServer.web(req, res, {}, (httpErr) => {
            if (httpErr) {
              // If both fail, return an error
              if (!res.headersSent) {
                res.writeHead(502, { "Content-Type": "text/plain" });
                res.end(`Service on port ${internalPort} is not available via either HTTP or HTTPS`);
              }
            }
          });
        }
      });
    };

    // Create HTTP server for the proxy
    const server = http.createServer(requestHandler);
    
    // Handle WebSocket connections
    server.on('upgrade', function (req, socket, head) {
      // Try HTTPS first, fallback to HTTP for WebSockets
      httpsProxyServer.ws(req, socket, head, (err) => {
        if (err) {
          httpProxyServer.ws(req, socket, head, (httpErr) => {
            if (httpErr) {
              socket.end();
            }
          });
        }
      });
    });

    server.on('error', (err) => {
      console.error(`Server error on port ${externalPort}:`, err.message);
    });

    // Start HTTP server
    server.listen(externalPort, '0.0.0.0', () => {
      console.log(`CORS proxy running on port ${externalPort} -> port ${internalPort} (trying HTTPS, fallback to HTTP)`);
      // Move to the next port after a small delay
      setTimeout(() => setupProxy(index + 1), 100);
    });
  } catch (err) {
    console.error(`Failed to set up proxy for port ${externalPort}:`, err.message);
    // Continue with next port even if this one fails
    setTimeout(() => setupProxy(index + 1), 100);
  }
};

// Start setting up proxies sequentially
setupProxy(0);
EOF

# Install dependencies for the CORS proxy
cd /opt/cors-proxy
npm install

# Create a systemd service for the CORS proxy
cat > /etc/systemd/system/cors-proxy.service << EOF
[Unit]
Description=CORS Proxy for Doppler Services
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/cors-proxy
Environment="HOME=/root"
# Use the exact path to node
ExecStart=/root/.nvm/versions/node/v22.14.0/bin/node /opt/cors-proxy/server.js
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=cors-proxy

[Install]
WantedBy=multi-user.target
EOF

# Open firewall ports if ufw is active
if command -v ufw &> /dev/null; then
  echo "Configuring firewall rules to allow traffic on ports 9090-10030 and 10090-11010..."
  ufw allow 80/tcp
  ufw allow 9090:10030/tcp # non-cors
  ufw allow 10090:11010/tcp # cors proxy
  ufw reload
fi

# Enable and start services
systemctl daemon-reload
systemctl enable haproxy
systemctl restart haproxy
systemctl enable cors-proxy
systemctl start cors-proxy

# Create systemd service for Doppler
echo "Setting up Doppler as a system service..."
cat > /etc/systemd/system/doppler.service << EOF
[Unit]
Description=Doppler Docker Compose Management Tool
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=root
WorkingDirectory=/root/.doppler/$DOPPLER_VERSION
Environment="HOME=/root"
Environment="NVM_DIR=/root/.nvm"
ExecStart=/bin/bash -c 'source /root/.nvm/nvm.sh && exec node ./build/'
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=doppler

[Install]
WantedBy=multi-user.target
EOF

# Enable and start Doppler service
systemctl daemon-reload
systemctl enable doppler
systemctl start doppler

# Verify installations
DOCKER_VERSION=$(docker --version)
COMPOSE_VERSION=$(docker compose version)
DOPPLER_STATUS=$(systemctl is-active doppler)
HAPROXY_STATUS=$(systemctl is-active haproxy)
CORS_PROXY_STATUS=$(systemctl is-active cors-proxy)

# Get public IP
PUBLIC_IP=$(curl -s http://169.254.169.254/metadata/v1/interfaces/public/0/ipv4/address)

echo "======================================================="
echo "Doppler installation complete!"
echo ""
echo "Docker: $DOCKER_VERSION"
echo "Docker Compose: $COMPOSE_VERSION"
echo "Doppler service: $DOPPLER_STATUS"
echo "HAProxy service: $HAPROXY_STATUS"
echo "CORS Proxy service: $CORS_PROXY_STATUS"
echo ""
echo "Access your Doppler instance at: http://$PUBLIC_IP"
echo "Username: $ADMIN_USER"
echo "Password: $ADMIN_PASS"
echo ""
echo "Services with CORS headers available on ports 10090-11010"
echo ""
echo "Please save these credentials securely."
echo "======================================================="
