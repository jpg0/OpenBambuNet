# OpenBambuNet Mock Printer

Mock Bambu Lab printer implementation for integration testing. Simulates a real printer's MQTT and FTP interfaces.

## Features

- **MQTT Broker**: Simulates printer MQTT interface on port 8883
  - Authentication (username: `bblp`, configurable password)
  - Subscribes to `device/{dev_id}/request` (commands from server)
  - Publishes to `device/{dev_id}/report` (status to server)
  - Auto-responds with status messages

- **FTP Server**: Simulates printer FTP interface on port 2121
  - Authentication (username: `bblp`, configurable password)
  - File upload tracking
  - In-memory file storage for verification

- **Control API**: REST API on port 3000 for test control
  - Inject custom messages
  - Verify received messages and uploaded files
  - Configure printer behavior
  - Reset state between tests

## Installation

```bash
npm install
npm run build
```

## Usage

### Start with defaults

```bash
npm start
```

### Start with custom configuration

```bash
DEV_ID=12345678 \
DEV_NAME="Test Printer" \
MODEL=3DPrinter-X1-Carbon \
PASSWORD=mypass \
MQTT_PORT=8883 \
FTP_PORT=2121 \
CONTROL_PORT=3000 \
npm start
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DEV_ID` | Printer device ID | `01234567` |
| `DEV_NAME` | Printer name | `Mock Printer` |
| `MODEL` | Printer model | `3DPrinter-X1-Carbon` |
| `PASSWORD` | Authentication password | `testpass` |
| `IP` | Printer IP address | `127.0.0.1` |
| `MQTT_PORT` | MQTT broker port | `8883` |
| `FTP_PORT` | FTP server port | `2121` |
| `CONTROL_PORT` | Control API port | `3000` |
| `USE_TLS` | Enable TLS (requires cert/key) | `false` |
| `CERT_PATH` | Path to TLS certificate | - |
| `KEY_PATH` | Path to TLS private key | - |

## Control API

The control API provides endpoints for managing the mock printer during tests.

### Health Check

```bash
curl http://localhost:3000/health
```

Response:
```json
{
  "status": "ok",
  "connected": true,
  "timestamp": "2025-11-22T09:00:00.000Z"
}
```

### Get Configuration

```bash
curl http://localhost:3000/config
```

### Update Configuration

```bash
curl -X POST http://localhost:3000/config \
  -H "Content-Type: application/json" \
  -d '{"devName": "New Name"}'
```

### Get Received Messages

```bash
curl http://localhost:3000/messages/received
```

Response:
```json
{
  "messages": [
    {
      "topic": "device/01234567/request",
      "payload": "{\"command\":\"test\"}",
      "timestamp": "2025-11-22T09:00:00.000Z"
    }
  ],
  "count": 1
}
```

### Inject Message (Publish to Report Topic)

```bash
curl -X POST http://localhost:3000/messages \
  -H "Content-Type: application/json" \
  -d '{"message": "{\"status\":\"printing\"}"}'
```

### Publish Status Message

```bash
curl -X POST http://localhost:3000/messages/status
```

### Clear Received Messages

```bash
curl -X DELETE http://localhost:3000/messages/received
```

### Get Uploaded Files

```bash
curl http://localhost:3000/files
```

Response:
```json
{
  "files": [
    {
      "filename": "test.gcode",
      "size": 12345,
      "timestamp": "2025-11-22T09:00:00.000Z"
    }
  ],
  "count": 1
}
```

### Get File Content

```bash
curl http://localhost:3000/files/test.gcode
```

Response includes base64-encoded file content.

### Clear Uploaded Files

```bash
curl -X DELETE http://localhost:3000/files
```

### Configure Auto-Respond

```bash
curl -X POST http://localhost:3000/auto-respond \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

### Reset All State

```bash
curl -X POST http://localhost:3000/reset
```

## Usage in Tests

### From Rust

```rust
use std::process::{Command, Child};
use std::time::Duration;
use tokio::time::sleep;

// Start mock printer
let mut child = Command::new("npm")
    .args(&["start"])
    .current_dir("../mock-printer")
    .spawn()
    .expect("Failed to start mock printer");

// Wait for startup
sleep(Duration::from_secs(2)).await;

// Run your tests...

// Cleanup
child.kill().expect("Failed to kill mock printer");
```

### Programmatic Usage

```typescript
import MockPrinter from './src/index';

const printer = new MockPrinter({
  devId: 'test123',
  mqttPort: 8884,
  ftpPort: 2122,
  controlPort: 3001,
});

await printer.start();

// Run tests...

await printer.stop();
```

## Development

### Run in development mode

```bash
npm run dev
```

### Build

```bash
npm run build
```

### Clean build artifacts

```bash
npm run clean
```

## Architecture

```
┌─────────────────────┐
│   Control API       │  HTTP REST API for test control
│   (Express)         │  Port: 3000
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│   Printer State     │  Shared state management
│   (EventEmitter)    │  - Configuration
└─────────┬───────────┘  - Messages
          │              - Files
          │
    ┌─────┴──────┐
    ▼            ▼
┌─────────┐  ┌─────────┐
│  MQTT   │  │   FTP   │
│ Broker  │  │ Server  │
│ (Aedes) │  │(ftp-srv)│
│Port:8883│  │Port:2121│
└─────────┘  └─────────┘
```

## License

AGPL-3.0 (same as bambu-farm)
