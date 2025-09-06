# AMP Server

An AI service proxy server with support for custom forwarding interfaces, featuring a clean architecture and flexible configuration system. Enhanced with production-ready features including health monitoring, error handling, API key validation, and Docker support.

## Architecture Overview

### Main Components

- **proxy module**: Unified proxy system supporting custom forwarding interfaces and headers
- **user module**: User-related mock endpoints
- **telemetry module**: Telemetry data collection endpoints
- **health module**: Health monitoring and system status endpoints
- **auth module**: API key authentication and authorization
- **error module**: Comprehensive error handling with retry mechanisms

### Unified Proxy System

The enhanced proxy system supports:
- Custom forwarding endpoints via YAML configuration files
- Custom request and response headers per endpoint
- Multiple response types: JSON, SSE, streaming, HTML
- Dynamic route registration
- Request timeout configuration
- Exponential backoff retry mechanisms
- API format conversion (OpenAI Responses ↔ Chat Completions, Google Gemini)

## Configuration

### Proxy Configuration File (proxy_config.yaml)

```yaml
# Global settings
global_timeout: 30  # seconds
global_retry:
  max_attempts: 3
  base_delay_ms: 100
  max_delay_ms: 10000
  backoff_multiplier: 2.0

endpoints:
  - path: "/api/provider/openai/v1/chat/completions"
    target_url: "https://api.openai.com/v1/chat/completions"
    method: "POST"
    response_type: "stream"
    timeout: 60  # Override global timeout for this endpoint
    custom_headers: {}
    forward_request_headers:
      - "authorization"
      - "content-type"
    forward_response_headers:
      - "content-type"
    enabled: true
```

### Environment Variables

- `HOST`: Server bind host (default: 127.0.0.1)
- `PORT`: Server port (default: 3000)
- `AMP_API_KEY`: Primary AMP service authentication key (required)
- `ADDITIONAL_API_KEYS`: Additional comma-separated API keys
- `RUST_LOG`: Log level (default: info)

## Usage

### Starting the Server

```bash
# Development
cargo run

# Production build
cargo build --release
./target/release/amp-server

# Docker
docker-compose up
```

### Docker Deployment

```bash
# Build and run with Docker Compose
docker-compose up -d

# Build Docker image only
docker build -t amp-server .

# Run with environment file
cp .env.example .env
# Edit .env with your configuration
docker-compose up
```

### Adding Custom Endpoints

1. Edit the `proxy_config.yaml` file
2. Add new endpoint configuration with optional timeout and retry settings
3. Restart the server

### Endpoint Configuration Parameters

- `path`: Local route path
- `target_url`: Target forwarding URL
- `method`: HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)
- `response_type`: Response type (json, sse, stream, html)
- `timeout`: Request timeout in seconds (overrides global)
- `retry`: Retry configuration (overrides global)
- `custom_headers`: Custom request headers
- `forward_request_headers`: List of request headers to forward
- `forward_response_headers`: List of response headers to forward
- `enabled`: Whether this endpoint is enabled

## API Endpoints

### Health Endpoints

- `GET /health` - Basic health check
- `GET /health/detailed` - Detailed system health information

### Proxy Endpoints (Configurable)

- `/api/provider/openai/v1/chat/completions` - OpenAI compatible interface
- `/api/provider/openai/v1/responses` - OpenAI Responses API
- `/api/provider/openai/v1/models` - OpenAI Models API
- `/api/provider/openai/v1/embeddings` - OpenAI Embeddings API
- `/api/provider/anthropic/v1/messages` - Anthropic compatible interface
- `/api/provider/google/v1/responses` - Google Gemini via Responses API
- `/api/provider/google/v1beta/*` - Google Gemini specific endpoints
- `/api/provider/xai/v1/chat/completions` - xAI (Grok) compatible interface
- `/api/provider/xai/v1/models` - xAI Models API
- `/api/provider/xai/v1/embeddings` - xAI Embeddings API
- `/api/provider/xai/v1/completions` - xAI Completions API
- `/api/provider/cerebras/v1/chat/completions` - Cerebras compatible interface
- `/api/provider/cerebras/v1/models` - Cerebras Models API
- `/api/provider/cerebras/v1/embeddings` - Cerebras Embeddings API
- `/api/provider/cerebras/v1/completions` - Cerebras Completions API
- `/api/tab/llm-proxy` - LLM proxy interface

### User Endpoints

- `GET /api/user` - Get user information
- `GET /api/connections` - Get connection list
- `POST /api/threads/sync` - Sync conversations
- `POST /api/internal` - Internal interface

### Telemetry Endpoints

- `POST /api/telemetry` - Send telemetry data

## Security Features

### API Key Authentication

- Automatic validation for protected endpoints (`/api/provider/*`, `/api/tab/llm-proxy`)
- Supports multiple authentication headers: `Authorization`, `X-API-Key`, `API-Key`
- Bearer token format support
- Multiple API key management

### Protected Paths

All `/api/provider/*` and `/api/tab/llm-proxy` endpoints require valid API key authentication.

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

### Lint

```bash
cargo clippy
```

## Production Features

### Error Handling & Resilience

- Exponential backoff retry mechanism
- Circuit breaker pattern for upstream failures
- Comprehensive error categorization
- Graceful degradation

### Monitoring & Observability

- Structured logging with file rotation
- Health check endpoints
- System metrics collection
- Request/response tracing

### Configuration Validation

- YAML configuration validation on startup
- Duplicate endpoint detection
- URL format validation
- HTTP method validation
- Header validation

## Features

- **Production-Ready**: Health checks, error handling, retries, validation
- **Secure**: API key authentication, request validation, secure headers
- **Configurable Proxying**: Easy setup of custom forwarding endpoints
- **Header Management**: Flexible request and response header configuration
- **Multiple Response Types**: Support for JSON, SSE, streaming, and HTML responses
- **Clean Architecture**: Modular design with clear separation of concerns
- **Mock Endpoints**: Built-in user and telemetry simulation endpoints
- **Environment Configuration**: Easy setup through environment variables
- **Docker Support**: Containerized deployment with docker-compose
- **Format Conversion**: Automatic API format conversion between providers

## API Format Conversions

The server automatically handles format conversions between different AI provider APIs:

### OpenAI Responses ↔ Chat Completions
- Converts o3 model requests from Responses API to Chat Completions format
- Converts streaming responses back to Responses format

### Google Gemini Integration
- Converts Responses API requests to Gemini generateContent format
- Supports both streaming and non-streaming modes
- Handles system instructions and conversation context

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