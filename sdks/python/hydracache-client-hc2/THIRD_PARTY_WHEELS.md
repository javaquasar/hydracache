# H15 offline Python wheelhouse

These unmodified wheels are retained only for the non-production HC/2 Python
generation gate. `wheelhouse.lock.json` fixes every filename, byte length, and
SHA-256 digest; `pip` also verifies the same hashes from `requirements.lock`
while running with `--no-index`.

| Distribution | Version | Upstream | License |
| --- | --- | --- | --- |
| grpcio | 1.76.0 | <https://pypi.org/project/grpcio/1.76.0/> | Apache-2.0 |
| protobuf | 6.33.4 | <https://pypi.org/project/protobuf/6.33.4/> | BSD-3-Clause |
| typing-extensions | 4.15.0 | <https://pypi.org/project/typing-extensions/4.15.0/> | PSF-2.0 |

Supported required environments are deliberately narrow:

- CPython 3.12 on glibc Linux x86-64 (GitHub-hosted required gate);
- CPython 3.13 on Windows x86-64 (current local development gate).

Other interpreters, architectures, musl, and macOS fail loud. Adding support
requires a separately reviewed wheel, hash, license entry, runtime row, and CI
execution; falling back to an sdist or network index is forbidden.
