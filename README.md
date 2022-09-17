# neosvr-simple-networking
Rust WebSocket Server for simple communication in NeosVR

Build as multiple producer, single consumer queue. Several queues can be created via the create api. The server does not store any data, it only forwards it to the websocket consumers.

## Available REST interfaces

* /v1/create
  Create a key pair for reading and writing to the other interfaces.
* /v1/post/{write_key}
  Send message via http body to the consumer.
* /v1/ws/{write_key}
  Send message via websocket to the consumer.
* /v1/ws/{read_key}
  Retrieve messages via websocket, only the last opened websocket for this read_key will retrieve the messages.
