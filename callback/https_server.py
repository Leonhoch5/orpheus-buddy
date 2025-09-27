import http.server
import ssl
import socketserver
import os
from pathlib import Path

# Get the directory where this script is located
SCRIPT_DIR = Path(__file__).parent.absolute()

class CallbackHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        # Set the directory to serve files from
        super().__init__(*args, directory=SCRIPT_DIR, **kwargs)
    
    def do_GET(self):
        # If accessing /callback, serve the index.html file
        if self.path.startswith('/callback'):
            self.path = '/index.html'
        
        # Call the parent method to serve the file
        super().do_GET()

def create_self_signed_cert():
    """Create a self-signed certificate for localhost"""
    try:
        import subprocess
        
        # Try common OpenSSL installation paths on Windows
        openssl_paths = [
            r"C:\Program Files\OpenSSL\bin\openssl.exe",
            r"C:\Program Files\OpenSSL-Win64\bin\openssl.exe",
            r"C:\Program Files (x86)\OpenSSL-Win32\bin\openssl.exe",
            r"C:\OpenSSL-Win64\bin\openssl.exe",
            "openssl"  # If it's in PATH
        ]
        
        openssl_cmd = None
        for path in openssl_paths:
            try:
                subprocess.run([path, "version"], capture_output=True, check=True)
                openssl_cmd = path
                print(f"✅ Found OpenSSL at: {path}")
                break
            except (subprocess.CalledProcessError, FileNotFoundError):
                continue
        
        if not openssl_cmd:
            raise FileNotFoundError("OpenSSL not found in common locations")
        
        # Create a self-signed certificate using openssl
        subprocess.run([
            openssl_cmd, 'req', '-x509', '-newkey', 'rsa:4096', '-keyout', 'key.pem', 
            '-out', 'cert.pem', '-days', '365', '-nodes', '-subj', 
            '/C=US/ST=State/L=City/O=Org/CN=127.0.0.1'
        ], check=True, cwd=SCRIPT_DIR)
        
        print("✅ Self-signed certificate created successfully!")
        return True
    except (subprocess.CalledProcessError, FileNotFoundError) as e:
        print(f"❌ OpenSSL not found or failed to create certificate: {e}")
        print("📥 Please add OpenSSL to your PATH or check installation")
        return False

if __name__ == "__main__":
    PORT = 8080
    
    cert_file = SCRIPT_DIR / 'cert.pem'
    key_file = SCRIPT_DIR / 'key.pem'
    
    # Check if certificates exist, create them if not
    if not cert_file.exists() or not key_file.exists():
        print("🔐 SSL certificates not found. Creating self-signed certificate...")
        if not create_self_signed_cert():
            print("Failed to create certificates. Exiting.")
            exit(1)
    
    # Create HTTPS server
    with socketserver.TCPServer(("127.0.0.1", PORT), CallbackHandler) as httpd:
        # Wrap the socket with SSL
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert_file, key_file)
        httpd.socket = context.wrap_socket(httpd.socket, server_side=True)
        
        print(f"🚀 Serving HTTPS at https://127.0.0.1:{PORT}/callback")
        print("⚠️  You'll need to accept the self-signed certificate warning in your browser")
        
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\n🛑 Server stopped")