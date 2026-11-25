"""prototypes/ 정적 서버: 수정 즉시 보이도록 캐시를 끈다."""
import http.server, os, sys
port = int(os.environ.get('PROTO_PORT', 8790))
root = os.path.join(os.path.dirname(__file__), '..', 'prototypes')
class H(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store'); super().end_headers()
handler = lambda *a, **k: H(*a, directory=root, **k)
print(f'http://127.0.0.1:{port}/', flush=True)
http.server.ThreadingHTTPServer(('127.0.0.1', port), handler).serve_forever()
