import http.server
import socketserver
import signal
import sys

def signal_handler(sig, frame):
    print("\nShutting down the server...")
    httpd.server_close()
    sys.exit(0)

PORT = 8003

# Set up the request handler
Handler = http.server.SimpleHTTPRequestHandler

# Create the server
httpd = socketserver.TCPServer(("", PORT), Handler)

# Set up the Ctrl+C signal handler
signal.signal(signal.SIGINT, signal_handler)

print(f"Serving at http://localhost:{PORT}")
httpd.serve_forever()