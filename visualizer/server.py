import http.server
import socketserver
import signal
import sys



def signal_handler(sig, frame):
    print("\nShutting down the server...")
    httpd.server_close()
    sys.exit(0)

PORT = 8006

class MyHTTPRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_my_headers()
        http.server.SimpleHTTPRequestHandler.end_headers(self)

    def send_my_headers(self):
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")

Handler = MyHTTPRequestHandler

# Create the server
httpd = socketserver.TCPServer(("", PORT), Handler)

# Set up the Ctrl+C signal handler
signal.signal(signal.SIGINT, signal_handler)

print(f"Serving at http://localhost:{PORT}")
httpd.serve_forever()

