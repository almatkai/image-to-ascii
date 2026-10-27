# QA and parent handoff

This is an **uncommitted development port**, not an installed or published release.
Source base: `f018cd7e93988a0c7e33d4afba54ea9d0ab6f49e`, branch
`feat/native-v2-rust`. The original V1 remains byte-identical except for a short
root README discovery link. The staged diff is empty. No commit, push, PR, release,
installation, profile mutation, ledger update or clone deletion was performed.

## Reproduce in this clone

`python3 v2/qa/verify.py --prepare` installs exact-lock **QA dependencies** with
scripts disabled into disposable clone-owned directories, snapshots own V1 at the
base SHA, regenerates and checks the unapplied patch, and runs the validations.
Use `python3 v2/qa/verify.py` when those dependencies already exist. All mutable
homes, temp files, fixture selections, npm/Cargo caches, build outputs, TLS and
archives belong to `v2/.qa` (the ignored development ZIP is directly under v2).
The script never installs a Rust target or OS tool. Fresh builds use an independent
empty Cargo home and the immutable public HTTPS SDK dependency. No product source
uses a sibling/path dependency, Node, Bun, JS engine, Tauri or host crate.

`python3 v2/qa/finalize.py` audits the byte boundaries and records final evidence
and the complete Git-visible SHA256 inventory, excluding only the inventory itself.
Do not stage or commit this handoff: the parent independently reviews and publishes.

## Executed coverage

- Stable and **Rust 1.85.0**: 14 Rust tests each (9 adapter/input/owned-TLS tests,
  2 exact pinned/proposed host-contract tests, 3 actual narrow producer tests).
  One ignored test is an intentionally explicit **test executable protocol helper**;
  it is separately executed by the OS-pipe harness. Both toolchains also pass
  fmt and strict all-target Clippy. No broad lint suppression covers product code.
- Closed source oracle: **180 assertions**, **30 numeric cases**, **14 response
  shapes**, **6 launch-selection cases**. It executes the untouched own TSX and
  locked Axios **1.15.2** Node distribution/HTTP adapter/default transformations,
  plus real form-data **4.0.5** serialization. Filesystem streams/stat calls are
  synthetic before module effects and callbacks; Axios has an inert fixture
  transport before any submit. Network/OS/browser/clipboard/settings/selection
  producers are denied or explicit inert mocks, including async stream and failure
  paths. No original file stream discovers the real filesystem. Source code,
  package and lock hashes are independently audited.
- Real OS-process JSONL pipes: **218 checks**, including typed all-field drafts,
  stale revisions/A/B/A, an invalid intermediate draft, repeat submits, Back,
  interrupted POST and a late reply, exact frozen-byte retry after the fixture file
  changes, ambiguous/canceled clipboard acknowledgements, repeated normal Copy,
  no-effect refresh, EOF, capacity/recovery and **140 canceled opens**. The helper's
  origin constructor is `cfg(test)` only and restricted to the owned localhost TLS
  endpoint. It is absent from the shipping binary.
- Owned TLS uses the **production reqwest transport**, real multipart parsers
  (multer and Python email MIME), fake bytes and local test certificates. Verified:
  field order/content/filename/MIME, optional dimensions, JS wire numeric values,
  plain text/JSON string/BOM, non-2xx, redirect rejection, non-text JSON, invalid
  UTF-8, invalid/untrusted TLS, plaintext peer, malformed/truncated HTTP, deadline,
  cancellation, declared/streamed oversize, and no automatic replay. No image was
  uploaded to the live service.
- Real React/JSDOM execution of pinned/proposed Form, Detail, Markdown,
  ExtensionRuntimeSurface and ExtensionRuntimeView: all-field snapshots,
  A/B/A revisions, stale responses, actual FilePicker callback, cancel/late picker,
  pending-submit guards, primary Detail callback, malicious ASCII output and
  unknown-copy notice. Native dialogs and OS services are inert; resource loading
  and clipboard calls remain zero. This is component QA, not GUI acceptance.
- Full pinned schema plus **15 negatives** (16 cases). Exact-lock disposable V1:
  **1 smoke test**, successful TypeScript typecheck. Root lock/package/source/
  original assets/metadata/license byte audits pass.
- Independent fresh HTTPS-SDK ARM64 release build, ignored development ZIP with
  closed hashes, safe inventory/extraction, and genuine **extracted production
  executable** smoke (7 checks). That executable runs under macOS sandbox-exec
  with external networking denied. It opens the normal form and validates an empty
  submit without uploading. Native test helper processes permit only localhost.
  ZIP and executable hashes are in [qa/evidence.json](qa/evidence.json).

The exact assertion counts, command exit codes and final hashes are recorded in
[qa/evidence.json](qa/evidence.json); the source response/numeric evidence is
[qa/oracle-cases.json](qa/oracle-cases.json). Raw execution logs remain ignored
under `.qa`. This file describes final checks, not every intermediate compiler fix.

## Deliberate compatibility differences and resource boundaries

This remains a client of the original fixed remote service. It does not implement
an alternative local converter. Source action order is retained: empty image,
existence/non-directory, extension, width then height, followed by stream upload.
Additional file/path/filename exclusions are explicit in [README.md](README.md).
Symlink/reparse paths, FIFOs/devices/directories and non-normal absolute paths are
rejected; Unix opens each component using directory handles with no-follow and
nonblocking flags, then checks identity/size/metadata again. Windows uses explicit
reparse checks and metadata validation; native Windows race/OS acceptance is not-run.
No selected-image bytes are interpreted, re-encoded or recompressed.

The native dimension parser preserves Number conversion and Number.toString for
fractions, exponent notation, radix forms, ECMAScript whitespace and subnormals.
Source Infinity and positive dimensions above 10,000 are **explicitly excluded**.
NaN, zero, negative values and underflow-to-zero are the same source validation
failures. The oracle tests precise expected messages and wire strings; native errors
are not accepted as generic parity success. Files are at most 8,000,000 bytes,
responses/copy text 1,000,000 bytes; the worst-case escaped Detail/copy frames fit
protocol limits. There is no truncation.

Axios automatically JSON-parses even text/plain. Plain/malformed-JSON text remains
text; JSON strings decode to their original value (including whitespace/newlines).
Valid non-string JSON (including overflow numeric JSON) is rejected, rather than
rendering an object's JavaScript coercion while copying a non-string value.
Unpaired UTF-16 surrogates in JSON strings are rejected because exact UTF-8 copy
is impossible. Invalid UTF-8 is rejected instead of Node's replacement decoding.
BOM removal follows the locked HTTP adapter. Source uses an unsafe fixed triple
fence; native output uses a longer safe fence and a generic React-escaped literal
Detail surface with no Markdown toolbar Copy bypass. Original copy text is retained
unchanged; backticks, HTML, Markdown, remote image syntax and bidi stay inert.

Network errors are static and sanitized, instead of exposing Axios response/path
messages. TLS verification is mandatory; system proxy, redirects, credentials and
automatic retries are disabled. Eight seconds bounds connect; 30 seconds bounds the
whole request. Any dispatched failure with uncertain outcome retains a frozen
snapshot and requires a deliberate retry. Non-2xx/invalid response failures are
known and return the same editable draft with a fresh submit action. Editing after
uncertainty abandons the previous retry snapshot and a new submit reads the current
file. No exactly-once, crash recovery or durable journal claim is made.

## UNAPPLIED host prerequisites and remaining acceptance

[integration/host-prerequisites.patch](integration/host-prerequisites.patch) is
against **ea100cee3f69711de12485ba061950c5fc65258f** only. It was checked against
exact copied Git-show fixtures, never applied to or built in a host checkout.
It changes only needed generic behavior: verified active-version resource lookup
independent of canonical preferences; exact declared aliases with platform/command
coverage and ambiguity rejection; explicit native single-file picker; all-field
form tokens/revisions and pending/late-picker guards; stale action/refresh response
fences; a Detail MouseEvent wrapper; and literal escaped Detail text.
The existing pinned `host.selection.read {kind:"files"}` producer already returns
an array of paths. Actual picker/selection/clipboard producer bodies are exercised
with surrounding OS/Tauri subprocess/dialog implementations mocked honestly.
No browser/hide/paste/window RPC is used; only selection.read and clipboard.write.

**macOS native GUI/Finder/clipboard, Windows native/GUI, and full-host acceptance:
not-run.** The available Windows GNU Rust target was attempted, but the default
build fails because `x86_64-w64-mingw32-gcc` is absent. A bounded attempt with the
existing Apple clang/ar fails in ring because Windows `assert.h` headers are absent.
No Windows link/native claim follows; MSVC tools/target were not installed. The
source manifest keeps the original macOS/Windows declaration with verification
states `not-run`, and `artifacts:{}`. No release URL, catalog or install was invented.

XERJ used canonical prefix `ax-ee7e25b773f5` and clone-owned `v2/.qa/xerj` state.
Initial source coverage was 4/4 code files (10 indexed files, 56 records, exit 3 for
three non-code junk/skips). Final indexing scanned 431 files and planned 223 new/changed files, then
aborted with exit 1 at v2/Cargo.toml because no frozen dataset accepts its docs
fields (unsupported schema evolution). The new v2 index is incomplete; initial
4/4 source-code coverage is the only completed coverage. Exact failure is recorded
in evidence. No alternate prefix, schema mutation or global restart was used.
Full reads, exact Git objects and hashes are authoritative where indexing is incomplete.
