# Test fixtures / Archives de test

Real RAR archives created with WinRAR by [`make-fixtures.ps1`](make-fixtures.ps1). Committed so CI does not need WinRAR.
Vraies archives RAR créées avec WinRAR par [`make-fixtures.ps1`](make-fixtures.ps1). Versionnées pour que la CI n'ait pas besoin de WinRAR.

| File / Fichier | Content / Contenu |
|---|---|
| `multi.part1..3.rar` | `payload.bin` (60 000 deterministic bytes / octets déterministes), stored, 20 KB volumes |
| `plain.rar` | `hello.txt` = `hello` |
| `password.rar` | `secret.txt` = `secret`, password / mot de passe `abc` |
| `headers.rar` | same, with encrypted headers / idem, en-têtes chiffrés (`-hp`) |
| `nested.rar` | `inner.rar` → `sub/inner.txt` = `inner` |
| `corrupt.rar` | `plain.rar` stored with one data byte changed → CRC error / un octet modifié → erreur CRC |
