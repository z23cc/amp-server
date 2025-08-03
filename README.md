# AMP Server

An AI service proxy server with support for custom forwarding interfaces, featuring a clean architecture and flexible configuration system after refactoring.

## Architecture Overview

### Main Components

- **proxy module**: Unified proxy system supporting custom forwarding interfaces and headers
- **user module**: User-related mock endpoints
- **telemetry module**: Telemetry data collection endpoints

### Unified Proxy System

The new proxy system supports:
- Custom forwarding endpoints via YAML configuration files
- Custom request and response headers per endpoint
- Multiple response types: JSON, SSE, streaming, HTML
- Dynamic route registration

## Configuration

### Proxy Configuration File (proxy_config.yaml)

```yaml
endpoints:
  - path: "/api/provider/openai/v1/chat/completions"
    target_url: "https://api.openai.com/v1/chat/completions"
    method: "POST"
    response_type: "stream"
    custom_headers: {}
    forward_request_headers:
      - "authorization"
      - "content-type"
    forward_response_headers:
      - "content-type"
    enabled: true
```

### Environment Variables

- `HOST`: Server bind host
- `PORT`: Server port
- `AMP_API_KEY`: AMP service authentication key
- `RUST_LOG`: Log level

## Usage

### Starting the Server

```bash
cargo run
```

### Adding Custom Endpoints

1. Edit the `proxy_config.yaml` file
2. Add new endpoint configuration
3. Restart the server

### Endpoint Configuration Parameters

- `path`: Local route path
- `target_url`: Target forwarding URL
- `method`: HTTP method (GET, POST, PUT, DELETE)
- `response_type`: Response type (json, sse, stream, html)
- `custom_headers`: Custom request headers
- `forward_request_headers`: List of request headers to forward
- `forward_response_headers`: List of response headers to forward
- `enabled`: Whether this endpoint is enabled

## API Endpoints

### Proxy Endpoints (Configurable)

- `/api/provider/openai/v1/chat/completions` - OpenAI compatible interface
- `/api/provider/anthropic/v1/messages` - Anthropic compatible interface
- `/api/tab/llm-proxy` - LLM proxy interface

### User Endpoints

- `GET /api/user` - Get user information
- `GET /api/connections` - Get connection list
- `POST /api/threads/sync` - Sync conversations
- `POST /api/internal` - Internal interface

### Telemetry Endpoints

- `POST /api/telemetry` - Send telemetry data

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Check

```bash
cargo check
```

## Features

- **Configurable Proxying**: Easy setup of custom forwarding endpoints
- **Header Management**: Flexible request and response header configuration
- **Multiple Response Types**: Support for JSON, SSE, streaming, and HTML responses
- **Clean Architecture**: Modular design with clear separation of concerns
- **Mock Endpoints**: Built-in user and telemetry simulation endpoints
- **Environment Configuration**: Easy setup through environment variables

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](#license) file for details.

---

## License

MIT License

Copyright (c) 2025 YougLin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.