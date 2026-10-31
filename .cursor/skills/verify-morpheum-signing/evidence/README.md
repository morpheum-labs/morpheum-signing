# Evidence

Drives write `evidence/<run-id>/`. Cleanup deletes `/tmp/verify-morpheum-signing-<run-id>` and does not delete this directory.

A passing drive leaves `result.txt` whose only line is `PASS`. The same line is in `<feature>/result.txt`. The seeded feature directory is `native-sign`. Stdout, stderr, the exit code, and the binary sha256 sit beside it.
