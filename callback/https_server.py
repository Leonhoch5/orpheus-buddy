import http.server
import socketserver
from urllib.parse import parse_qs, urlparse
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent.absolute()
AUTH_CODE_FILE = SCRIPT_DIR / "hackclub_auth_code.txt"

class CallbackHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SCRIPT_DIR, **kwargs)
    
    def do_GET(self):
        parsed = urlparse(self.path)

        if parsed.path == '/callback':
            code = parse_qs(parsed.query).get('code', [''])[0]
            if code:
                AUTH_CODE_FILE.write_text(code, encoding='utf-8')
            self.path = '/index.html'

        elif parsed.path == '/hackclub-auth-code':
            code = AUTH_CODE_FILE.read_text(encoding='utf-8') if AUTH_CODE_FILE.exists() else ''
            self.send_response(200)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.send_header('Cache-Control', 'no-store')
            self.end_headers()
            self.wfile.write(code.encode('utf-8'))
            return

        elif parsed.path == '/hackclub-auth-code-clear':
            if AUTH_CODE_FILE.exists():
                AUTH_CODE_FILE.unlink()
            self.send_response(204)
            self.end_headers()
            return

        super().do_GET()

if __name__ == "__main__":
    PORT = 3001

    with socketserver.TCPServer(("localhost", PORT), CallbackHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}/callback")
        
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped")