# chat-app
Chat app with rust 


this is the end goal; Learning as I go.
------------------------------------------------------------------
Rust P2P Messenger (Concept)

A privacy-first, peer-to-peer messenger written in Rust.

No central servers → all traffic goes peer-to-peer over Tor hidden services.

End-to-end encryption → messages are encrypted with per-chat session keys.

User-controlled identity → each user shares a QR code containing their .onion address + public key.

------------------------------------------------------------------

Phase 1 — Barebones Chat

Simple TCP client/server in Rust.

Plaintext messages over LAN.


Phase 2 — Encryption Layer

Add end-to-end encryption using snow (Noise Protocol).

Keypairs per user, session keys per chat.

Phase 3 — Peer-to-Peer

Remove central server; direct client ↔ client connections.

LAN/port-forwarding tests for NAT traversal.


Phase 4 — Tor Integration

Route traffic via Tor (SOCKS5 proxy).

Replace IPs with .onion addresses.


Phase 5 — Identity Exchange UX

Bundle onion address + public key into QR codes.

Add scanner/import flow for contacts.


Phase 6 — Reliability Extras

Local encrypted storage for chat history.

Optional relays for offline message delivery.

File transfer & group chat experiments.


Phase 7 — Mobile Packaging

Wrap Rust core for Android/iOS (via React Native, Flutter, or native).

First release as a privacy-first mobile messenger.