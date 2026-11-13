# Local patches to `unrar` 0.5.8

Upstream: https://github.com/muja/unrar.rs (MIT OR Apache-2.0).

## 1. Out-of-bounds read on volume change

`Internal::<M>::callback` handled `UCM_CHANGEVOLUMEW` with
`WideCString::from_ptr_truncate(p1, 2048)`, which always reads 2048 UTF-16 units
from the buffer passed by UnRAR. That buffer is usually much shorter, so the read
can run past the end of the allocation and crash the process with
`STATUS_ACCESS_VIOLATION` (observed ~25% of runs on multi-volume archives).

Fix: `read_wide_nul` scans for the terminating NUL (bounded by 2048) before copying.
Location: `src/open_archive.rs`.
