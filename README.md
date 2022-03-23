![Continuous Integration](https://github.com/helium/semtech-udp/workflows/Continuous%20Integration/badge.svg)

# Semtech UDP

Semtech UDP provides serialization and deserialization of packets complying 
with the Semtech GWMP over UDP protocol.

The `server` feature provides a Tokio-based runtime which handles the UDP and
Semtech GWMP UDP protocol details, such as ACKs and keeping track of client
addresses. It exposes an async API for receiving all messages for the client
and an asynchronous send function which returns only when the transmit ack
(tx_ack) is received

The `client` feature provides a Tokio-based runtime which handles the UDP and
Semtech UDP protocol details, such as periodically sending PULL_DATA frames.
Client is responsible for ACKing downlinks.

## Usage

Please see the examples for usage. This library is used in [gateway-rs](https://github.com/helium/gateway-rs)
and [gwmp-mux](https://github.com/helium/gwmp-mux).
