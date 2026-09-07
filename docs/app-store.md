# Application store

The store is maintained in an independent repository. Its publisher writes immutable catalog snapshots, application media and installation artifacts, then conditionally replaces the static current pointer. Studio reads public HTTP objects; it has no D1, R2 or signing credentials.

## Default source and local development

Native Studio builds use `https://studio-store.pocket.nexus/` by default. The repository identity and trusted Ed25519 public key are embedded from `src-tauri/src/infrastructure/store/official-source.json`; users do not need a configuration file or environment variable. Only public trust material is bundled. Changing the built-in trust requires a new Studio build.

`POCKET_STORE_CONFIG` optionally replaces the entire default source, including its trusted keys. An unreadable or malformed override reports an error rather than silently switching back to the official source.

To use a local catalog, initialize and publish it in the store repository, run its `client-config` command, and start its static preview server. Use the absolute configuration path printed by that command when starting Studio:

```sh
POCKET_STORE_CONFIG=/absolute/path/to/store/.local/studio-source.json pnpm tauri dev
```

The source configuration contains `base_url`, `allow_local_http` and a `trust` object with the repository ID and trusted Ed25519 public keys. HTTP is accepted only for an explicitly enabled loopback source. Public HTTPS sources use the normal TLS verifier. Browser preview continues to use the isolated device/catalog simulator.

## Protocol and cache

The versioned contract is pinned in `contracts/store-v1`. `pnpm sync:store-protocol` generates frontend types from that copy; `--source` can update it from the store repository. Runtime access does not depend on another checkout.

Rust verifies the catalog's exact UTF-8 bytes, SHA-256, signature, repository identity and monotonically increasing sequence. A different catalog at an already accepted sequence is rejected. Only verified data reaches the webview projection. Application identity, release, artifact, native identity and target requirements remain separate models.

The native cache stores catalog records and download indexes in SQLite. Complete and partial objects are stored separately by SHA-256. Downloads use an OS lock across Studio processes, validate Range/If-Range responses, flush interrupted partial files and verify complete bytes before returning an installable file. The asset protocol exposes only the store's blob directory; media must be referenced by the verified catalog.

A failed refresh preserves the most recent verified catalog and reports the reason. An expired catalog remains browseable, but cannot resolve a new installation. Offline installation requires an unexpired verified catalog and a complete verified artifact. Unlisted applications and withdrawn releases retain their identity and history for installed-application management.

## Verification

Native tests cover the publisher's signature fixture, compatibility selection, expiry, rollback and equivocation rejection, persistent cache recovery, partial downloads, invalid bytes and cancellation. HTTP tests use loopback servers and synthetic blobs, not physical devices. Frontend tests cover metadata-driven localization and target-specific media selection. Physical installation tests must be started explicitly and reported separately.

The installed page queries only native bundle IDs present in the trusted catalog,
including withdrawn releases. It reads separate product versions, build numbers
and bounded build receipts through the native library. Sideloaded apps are mapped
by native identity; a revision is known only when version, build and receipt all
match an artifact. Multiple native identities remain distinct observations.

SQLite observations are scoped to the repository and an opaque device key. Failed
reads return stale or unavailable state; they never replace a prior observation
with an empty list. A successful empty response does replace it. The UI shows the
observation time, download size and unknown revisions without inventing install
dates or installed storage usage. Device writes require the explicit operation
plan and consent described below.

## Native application operations

`plan_package` resolves a device-bound install, update, reinstall or uninstall
plan. `start_package` consumes that plan once; uninstall requires explicit data
removal consent. Updates require a recognized store receipt, and native product
versions and build numbers prevent downgrades. Unknown revisions use explicit
reinstall through the system Upgrade command. System applications are rejected.

Rust owns a FIFO application queue. It shares a write semaphore with device
preparation and currently serializes device writes across the desktop process.
Plans retain their selected artifact and physical identity. Execution refreshes
the catalog and device state before downloading and before device I/O; publishing
a different release never substitutes an artifact in an accepted plan. Withdrawal,
expiry, changed installed state, checksum mismatch and missing prerequisites stop
the operation. Unknown AppSync or jailbreak observations permit a normal User
installation; the plan explains that uncertainty. AppSync package recognition
uses its current/legacy IDs and the `Provides: appsync` capability documented by
[AppSync's package metadata](https://github.com/akemin-dayo/AppSync/blob/master/control).

SQLite stores the accepted plan, device binding, queue order, staging path,
submission state and result. Submission intent is durably recorded before the
library commits a system request. Cancellation shares that transition's lock.
After submission it is refused; transport uncertainty leads to observation rather
than replay. Registration, product/build versions and the build receipt are read
back before success is reported. A missing observation is “verification incomplete”.
`verify_package_operation` runs only read-only inspection on the original device.

Startup marks persisted active records interrupted. It never resumes device
writes. `list_package_operations` restores webview state, including the original
device, application names, version and action. Window close and application exit
are blocked while queued or active operations remain. Staging cleanup failure is
separate from the device installation result. Unknown system progress is shown
as working, without inventing a percentage.

Native operation tests cover consent, FIFO exclusion, preparation exclusion,
queued and transfer cancellation, noncancellable submission, publication withdrawal,
state changes, signature rejection, persistence failure, interruption recovery,
downgrade prevention and read-only re-verification. They use protocol/adapter
fixtures and do not replace user-initiated hardware acceptance.

## Verify a locally published application

The read-only `verify_store` example runs the production Rust catalog, media and
artifact readers without opening any device service:

Omit `POCKET_STORE_CONFIG` to check the built-in official source, or set it to check a local publication:

```sh
POCKET_STORE_CONFIG=/absolute/path/to/store/.local/studio-source.json \
  cargo run --manifest-path src-tauri/Cargo.toml --example verify_store -- \
  APP_ID /absolute/path/to/store/.local/studio-check-cache
```

With the publisher's preview server running, it verifies the signed catalog,
artifact digest and native IPA identity, plus referenced media. Stop the preview
server and repeat the command to verify the same complete cache offline. A live
local publication was checked with the application `dev.pocket-stack.clear`,
product version `0.1.0`, revision/native build `1`, and host ABI `8`. Its real
984988-byte IPA and icon passed both online and offline checks. This is a host
integration check; device installation acceptance remains user-initiated.

## AppSync preparation

Device preparation now includes AppSync and its required system packages. An already jailbroken device gets a separate USB SSH plan, with no DFU or jailbreak steps. The user reviews package changes and supplies the device root password before explicitly starting. Missing AppSync blocks installation; an unreadable status remains unknown. Device signature rejection links to the environment page and explains AppSync instead of guessing that disk space is low. See [native preparation](native-preparation.md) for pinned package sources, credential handling, cancellation and remaining hardware validation.
