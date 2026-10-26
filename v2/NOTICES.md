# Notices and provenance

Own V1 code/license: `almatkai/image-to-ascii` at
`f018cd7e93988a0c7e33d4afba54ea9d0ab6f49e`; [original MIT license](../LICENSE),
copyright (c) 2025 Yen Cheng Lin. This port retains it as [LICENSE](LICENSE).

Original Raycast extension/icon: `raycast/extensions` at
`6734d85992290066fd31a7f5bee3cca3d3545a1a`, path
`extensions/image-to-ascii`. [Original Raycast MIT license](resources/RAYCAST-LICENSE).
The exact downloaded icon SHA256 is
`883d6bb2858722b70c6d72dc47b81dc52790cb7bab256138347e0f1c7e54660e`.
It equals own `assets/extension-icon.png` and [resources/extension-icon.png](resources/extension-icon.png).
No generated or recompressed icon is used. No additional original asset is used
by the command; V1 metadata screenshots remain untouched.

SDK `tezbar-extension-sdk` 0.1.1 is pinned over HTTPS Git at
`e955911f0bf5b558f9d3eb5183df0adc95c22af2`; authentic Apache 2.0
[license](resources/SDK-LICENSE) and NOTICE are retained under dependency notices.
Host fixtures are exact Git-show bytes at
`ea100cee3f69711de12485ba061950c5fc65258f`; [host license](qa/fixtures/host/LICENSE).

The generic prerequisite generator, narrow host/producer/React test harnesses,
packaging/fresh-build/notice scripts and malformed archive fixtures were adapted
from `almatkai/image-base64` at immutable
`51e302cacdaeea11038704ef2825893cb714e1f3` (MIT). Its authentic license is
[qa/fixtures/IMAGE-BASE64-LICENSE](qa/fixtures/IMAGE-BASE64-LICENSE).
No clone or branch/path dependency on that repository is used.

Authentic locked Cargo dependency licenses/notices are copied unmodified under
[resources/licenses](resources/licenses). [qa/dependency-notices.json](qa/dependency-notices.json)
records exact package versions, sources, SPDX expressions and copied hashes,
including dev/platform dependencies. Packages distributing no separate notice are
explicitly listed without fabricated text. Renderer tools have a separate exact
npm lock and do not ship in the product. TLS certificates and the committed
private key are synthetic QA-only localhost fixtures; they confer no production
trust and never enter the development product ZIP. Fake selected image bytes are
locally created inert test data. All fixture URL/hash pins appear in [qa/pins.json](qa/pins.json).
