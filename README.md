# Semtech UDP

Semtech UDP provides serialization and deserialization of packets complying
with the Semtech UDP protocol.

The **server** feature provides a Tokio-based runtime which handles  all the
UDP and Semtech UDP protocol details, such as ACKs and keeping track of client
addresses. 

It exposes an async API for receiving all messages for the client
and an asynchronous send function which returns only when the transmit ack is
received.