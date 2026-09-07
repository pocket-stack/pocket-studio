# Pocket Store protocol v1

These schemas and the interoperability fixture are pinned copies of the protocol owned by the independent Pocket Store repository. Studio verifies the raw catalog bytes with the trusted Ed25519 public key before parsing application data.

To update the copies and generate their frontend types, run:

```sh
pnpm sync:store-protocol --source /absolute/path/to/store/protocol
```

Running without `--source` regenerates types from the checked-in contract. Building or running Studio does not require a sibling checkout. Native tests verify the publisher's UTF-8 signature fixture without serializing JSON again. The fixture uses a public test identity and is never a default trusted source.
