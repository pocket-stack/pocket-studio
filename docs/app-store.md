# Application store

The store is maintained in an independent repository. Its publisher writes immutable catalog snapshots, application media and installation artifacts, then conditionally replaces the static current pointer. Studio reads public HTTP objects; it has no D1, R2 or signing credentials.

## Local development

Initialize and publish a catalog in the store repository, run its `client-config` command, and start its static preview server. Use the absolute configuration path printed by that command when starting Studio:

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
