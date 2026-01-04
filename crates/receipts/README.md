# asga-receipts

Canonical receipt types and SCALE serialization for the Atomic Swap Guardian Agent (ASGA).

Includes:
- `ReceiptHeader` (common header)
- Domain payload types: `EvmReceipt`, `SvmReceipt`, `BtcReceipt`, `X3Receipt`
- Deterministic encode/decode unit tests

Next:
- Add signature verification helpers
- Add canonical COSE/JWS mapping or adaptor for existing domain proofs
- Integrate into `pallet-atomic-guardian`
