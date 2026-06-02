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
HACKCLUB_CODE_FILE = AUTH_DIR / 'hackclub_auth_code.txt'
SLACK_CODE_FILE = AUTH_DIR / 'slack_auth_code.txt'

class CallbackHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SCRIPT_DIR, **kwargs)
    
    def do_GET(self):
        parsed = urlparse(self.path)

        if parsed.path == '/callback':
            code = parse_qs(parsed.query).get('code', [''])[0]
            state = parse_qs(parsed.query).get('state', [''])[0]
            print(f"Callback received: path=/callback code={code} state={state}")
            if code:
                # write to provider-specific file based on state param (loose match)
                if 'slack' in state.lower():
                    SLACK_CODE_FILE.write_text(code, encoding='utf-8')
                    print(f"Wrote Slack code to {SLACK_CODE_FILE}")
                else:
                    HACKCLUB_CODE_FILE.write_text(code, encoding='utf-8')
                    print(f"Wrote Hack Club code to {HACKCLUB_CODE_FILE}")
            self.path = '/index.html'

        elif parsed.path == '/hackclub-auth-code':
            # allow query param to write code as fallback
            qp = parse_qs(parsed.query)
            if 'code' in qp and qp['code']:
                code_val = qp['code'][0]
                HACKCLUB_CODE_FILE.write_text(code_val, encoding='utf-8')
                print(f"/hackclub-auth-code: wrote code from query to {HACKCLUB_CODE_FILE}")
            code = HACKCLUB_CODE_FILE.read_text(encoding='utf-8') if HACKCLUB_CODE_FILE.exists() else ''
            self.send_response(200)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.send_header('Cache-Control', 'no-store')
            self.end_headers()
            self.wfile.write(code.encode('utf-8'))
            return

        elif parsed.path == '/hackclub-auth-code-clear':
            if HACKCLUB_CODE_FILE.exists():
                HACKCLUB_CODE_FILE.unlink()
            self.send_response(204)
            self.end_headers()
            return

        elif parsed.path == '/slack-auth-code':
            # allow query param to write code as fallback
            qp = parse_qs(parsed.query)
            if 'code' in qp and qp['code']:
                code_val = qp['code'][0]
                SLACK_CODE_FILE.write_text(code_val, encoding='utf-8')
                print(f"/slack-auth-code: wrote code from query to {SLACK_CODE_FILE}")
            code = SLACK_CODE_FILE.read_text(encoding='utf-8') if SLACK_CODE_FILE.exists() else ''
            self.send_response(200)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.send_header('Cache-Control', 'no-store')
            self.end_headers()
            self.wfile.write(code.encode('utf-8'))
            return

        elif parsed.path == '/slack-auth-code-clear':
            if SLACK_CODE_FILE.exists():
                SLACK_CODE_FILE.unlink()
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