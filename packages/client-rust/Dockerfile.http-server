# Multi-stage build for smaller final image
FROM rust:1.89 as builder

WORKDIR /app
COPY . .

# Build the HTTP server
RUN cargo build --release --bin http_server

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/http_server /app/http_server

# Make sure the binary is executable
RUN chmod +x /app/http_server

# Expose the port
EXPOSE 4399

# Set environment variables
ENV RUST_LOG=info

# Run the server
CMD ["./http_server"]
