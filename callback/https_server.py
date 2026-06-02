import http.server
import socketserver
from urllib.parse import parse_qs, urlparse
from pathlib import Path
import os
import sys

SCRIPT_DIR = Path(__file__).parent.absolute()

def app_data_dir():
    system = os.name
    # Windows: %APPDATA%
    if os.name == 'nt':
        base = os.environ.get('APPDATA')
        if base:
            return Path(base) / 'orpheus-buddy'
    # macOS
    if sys.platform == 'darwin':
        return Path.home() / 'Library' / 'Application Support' / 'orpheus-buddy'
    # Linux / other: XDG_DATA_HOME or ~/.local/share
    xdg = os.environ.get('XDG_DATA_HOME')
    if xdg:
        return Path(xdg) / 'orpheus-buddy'
    return Path.home() / '.local' / 'share' / 'orpheus-buddy'

AUTH_DIR = app_data_dir()
AUTH_DIR.mkdir(parents=True, exist_ok=True)
AUTH_CODE_FILE = AUTH_DIR / 'hackclub_auth_code.txt'

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