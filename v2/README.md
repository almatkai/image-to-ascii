# Image to Ascii — native Rust v2

Development-only standalone SDK 0.1.1 / schema 3 / protocol 1 port of the own
repository's single `convert-image-to-ascii` command. The original V1 files remain
unchanged. There are no published artifacts, catalog entries or installation steps.

Select one JPEG/JPG/PNG/BMP/WebP image, optionally enter width/height, choose
Complex, then submit. **Submit uploads the original file to
`https://ascii-image-converter.zeabur.app/image-to-ascii`.** The service performs
conversion. One allowed Finder/Explorer selection is used as a draft prefill;
selection failure leaves the editable form. Width defaults to `80`; height is
empty. Supplying only one dimension asks the service to preserve aspect ratio.

The returned text opens a literal Detail view. Copy Ascii Image sends the exact
returned text; Back returns the same draft. Copy and upload actions are consumed
before awaiting external work. A canceled/uncertain upload offers **Retry Previous
Upload**, which sends the frozen bytes/filename/options again and may repeat
service work. Editing the form abandons that retry snapshot; a new submit reads
the current selected file. Copy uncertainty offers an exact-text retry without
another upload. No automatic replay, history, preferences or persistence is added.
These checkpoints are in memory, not crash-durable or an exactly-once guarantee.

Explicit resource/security differences: ordinary absolute UTF-8 paths only,
4096-byte path and 512-byte filename bounds, no symlink/reparse components,
FIFO/device/directory inputs, or filenames containing controls/quotes/backslashes.
Files are at most 8,000,000 bytes; dimensions use JavaScript Number semantics
(including fractions, exponents and unsigned radix forms) but must additionally be
finite and at most 10,000. Dimension fields are at most 256 UTF-8 bytes.
Unpaired Unicode surrogates in JSON strings are rejected because exact UTF-8 copy is impossible.
Responses/copy text are bounded at 1,000,000 bytes, validated UTF-8, and never
truncated. JSON strings are decoded as Axios does; JSON objects/arrays/numbers/
booleans/null are rejected instead of coercing non-text output to ASCII.
The transport uses verified TLS, bundled WebPKI roots, no system proxy, cookies,
credentials, redirects or retries, an 8-second connect limit and 30-second overall
deadline. Errors show static text, without paths or server response dumps.

## Build and review

Rust stable and 1.85.0 are supported. The immutable HTTPS SDK revision is
`e955911f0bf5b558f9d3eb5183df0adc95c22af2`; there is no host/path dependency or JS
engine in the product. Build with `cargo build --locked --manifest-path v2/Cargo.toml`.
For fully confined reproducible QA see [QA.md](QA.md). Do not upload a real image
for QA. All generated data/cache/build output belongs under `v2/.qa`.

The pinned host **needs the UNAPPLIED generic prerequisite patch** at
[integration/host-prerequisites.patch](integration/host-prerequisites.patch).
It supplies version-aware resources, exact verified alias routing, explicit
native file picking, all-field/revision/pending form guards, Detail callback
wrapping and literal inert text. Native file selection already exists at the
host pin. [QA.md](QA.md) identifies what was tested and what remains not-run.
No host working tree has been modified or built. Native Windows, GUI and full-host
acceptance are not claimed; the declared platforms preserve the source metadata.

See [NOTICES.md](NOTICES.md), [qa/pins.json](qa/pins.json),
[qa/evidence.json](qa/evidence.json) and the full Git-visible [INDEX.sha256](INDEX.sha256).
