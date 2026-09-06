# ADR-0003: Serial and network transport abstraction

- Status: Accepted
- Decision: Define transport-independent diagnostic layers above future `SerialTransport` and `NetworkTransport` adapters.
- Reason: Desktop can connect directly or through a bridge, while mobile uses the network bridge.
- Consequence: No protocol or JLR knowledge package may assume a COM port or socket implementation.
