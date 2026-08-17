# Code Coverage Tracking

SkyOS does not currently track code coverage. There is no LLVM coverage instrumentation, no `grcov` setup, and no coverage gate in CI.

The `integration-qemu` CI job's TAP gate (kernel `self_test`) and the QEMU boot-to-login check are the closest functional equivalent — they verify that boot-critical paths execute, but they do not report line/function coverage percentages.

If coverage is added, it would apply to:
- Host-side suites (`tests/skyos-test`) — trivially instrumentable since they run under std
- Kernel self-tests — requires running instrumented kernel in QEMU and merging the profile

Any "current coverage %" figures in older revisions of this document were aspirational, not measured.
