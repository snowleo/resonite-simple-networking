# resonite-simple-networking
Rust WebSocket Server for simple communication in Resonite

Build as multiple producer, single consumer queue. Several queues can be created via the create api. The server does not store any data, it only forwards it to the websocket consumers.

## Available REST interfaces

* `GET /v1/create`
  Create a key pair for reading and writing to the other interfaces.
* `POST /v1/post/{write_key}`
  Send message via http body to the consumer.
* `WS /v1/ws/{write_key}`
  Send message via websocket to the consumer.
* `WS /v1/ws/{read_key}`
  Retrieve messages via websocket, only the last opened websocket for this read_key will retrieve the messages.

## Launching

Expected to run as a systemd process. Example:

```
[Unit]
Description=resonite-simple-networking daemon
After=syslog.target network.target

[Service]
ExecStart=/path/to/resonite-simple-networking
KillMode=process
Restart=on-failure
RestartSec=42s
User=user
PassEnvironment=RUST_LOG
Environment=RUST_LOG=info
LoadCredential=TLS_KEY:/etc/letsencrypt/live/domain/privkey.pem
LoadCredential=TLS_CERT:/etc/letsencrypt/live/domain/fullchain.pem
LoadCredential=ENCRYPTION_KEY:/etc/resonite.key

[Install]
WantedBy=multi-user.target
```
