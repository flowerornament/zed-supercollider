# HTTP API Reference

The SuperCollider launcher exposes a local HTTP API for code evaluation and server control. This API is used by Zed tasks and can be used by external tools.

## Server Details

- **Host:** `127.0.0.1` (localhost only, not accessible from network)
- **Port:** `57130` (default, configurable via `--http-port`)
- **CORS:** Enabled for browser-based tools

## Endpoints

### POST /eval

Evaluate SuperCollider code.

**Request:**
- Content-Type: `text/plain`
- Body: SuperCollider code to evaluate

**Response:**
- `202 Accepted` - Code submitted for evaluation

**Behavior:**
- Fire-and-forget: The response returns immediately, before evaluation completes
- Results appear in the sclang post window log (`$TMPDIR/sclang_post.log`)
- Large payloads (>6000 bytes) are automatically chunked over UDP

**Example:**
```bash
curl -X POST -H "Content-Type: text/plain" \
  -d "{ SinOsc.ar(440) }.play" \
  http://127.0.0.1:57130/eval
```

### POST /boot

Boot the SuperCollider audio server.

**Response:** `202 Accepted`

**Example:**
```bash
curl -X POST http://127.0.0.1:57130/boot
```

### POST /stop

Stop all sounds (equivalent to Cmd+Period / CmdPeriod).

**Response:** `202 Accepted`

**Example:**
```bash
curl -X POST http://127.0.0.1:57130/stop
```

### POST /quit

Quit the SuperCollider audio server.

**Response:** `202 Accepted`

**Example:**
```bash
curl -X POST http://127.0.0.1:57130/quit
```

### POST /recompile

Recompile the SuperCollider class library.

**Response:** `202 Accepted`

**Example:**
```bash
curl -X POST http://127.0.0.1:57130/recompile
```

### GET /health

Health check endpoint.

**Response:**
- `200 OK` with body `{"status":"ok"}`

**Example:**
```bash
curl http://127.0.0.1:57130/health
```

### POST /convert-schelp

Convert a .schelp help file to markdown. Used for hover documentation.

**Request:**
- Content-Type: `application/json`
- Body: `{"path": "/path/to/File.schelp"}`

**Response:**
- `200 OK` with body `{"markdown": "..."}`
- `400 Bad Request` - missing path or invalid JSON
- `404 Not Found` - file doesn't exist
- `500 Internal Server Error` - pandoc not available or conversion failed

**Example:**
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"path": "/Applications/SuperCollider.app/Contents/Resources/HelpSource/Classes/SinOsc.schelp"}' \
  http://127.0.0.1:57130/convert-schelp
```

**Note:** Requires pandoc to be installed and `tools/schelp/schelp.lua` to be accessible.

## Error Handling

All POST endpoints return `202 Accepted` on successful submission. If the underlying UDP communication fails (e.g., sclang not responding), the endpoint returns:

- `502 Bad Gateway` - failed to communicate with sclang
- `400 Bad Request` - malformed request body

## Notes

- The HTTP server is localhost-only for security
- All control commands are asynchronous (fire-and-forget)
- Evaluation results are not returned in the HTTP response; check the post window log
- The launcher handles UDP message chunking transparently for large payloads
