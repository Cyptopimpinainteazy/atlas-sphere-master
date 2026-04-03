#!/usr/bin/env bash
# tools/ollama/test_generate_md.sh
# Simple integration test for generate_md.sh using a mocked Ollama HTTP server

set -euo pipefail

PORT=9555
HOST="http://127.0.0.1:$PORT"
TMP_PROMPT=$(mktemp)
TMP_OUT=$(mktemp -u --suffix=.md)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cat > "$TMP_PROMPT" <<EOF
Test prompt: please produce a deterministic short message for testing.
EOF

# Start a tiny Python HTTP server that responds to POST /api/generate with a
# predictable JSON payload and responds to GET / for liveness probes.
python3 - <<PYTHON &
import http.server, socketserver, json, sys
class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path != '/api/generate':
            self.send_response(404)
            self.end_headers()
            return
        length = int(self.headers.get('Content-Length', 0))
        _ = self.rfile.read(length)
        resp = {"results":[{"content":[{"text":"This is test output"}]}]}
        resp_bytes = json.dumps(resp).encode('utf-8')
        self.send_response(200)
        self.send_header('Content-Type','application/json')
        self.send_header('Content-Length', str(len(resp_bytes)))
        self.end_headers()
        self.wfile.write(resp_bytes)
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type','text/plain')
        self.end_headers()
        self.wfile.write(b'ok')
    def log_message(self, format, *args):
        return

with socketserver.TCPServer(("127.0.0.1", $PORT), Handler) as httpd:
    print('mock server listening', file=sys.stderr)
    httpd.serve_forever()
PYTHON

SERVER_PID=$!
# Wait for server to be ready
for i in {1..20}; do
  if curl -s "$HOST" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

# Run the generator against the mock server
chmod +x "$SCRIPT_DIR/generate_md.sh" || true
"$SCRIPT_DIR/generate_md.sh" -p "$TMP_PROMPT" -o "$TMP_OUT" -h "$HOST" -m "test-model"

if ! grep -q "This is test output" "$TMP_OUT"; then
  echo "ERROR: expected text not found in $TMP_OUT" >&2
  kill $SERVER_PID || true
  exit 1
fi

echo "Test passed: output contains expected content"

kill $SERVER_PID || true
rm -f "$TMP_PROMPT" "$TMP_OUT"
