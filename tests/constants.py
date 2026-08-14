#!/usr/bin/env python3
"""Shared source-contract constants for the host-runnable test suites.

MAX_RESPAWNS is the SINGLE authoritative Python mirror of
`const MAX_RESPAWNS: u32 = 5;` in init/src/main.rs. The source-agreement
pins test_vahid_contract.py::test_port_matches_source_max_respawns and
test_init_accumulates_crashes_and_gives_up, plus
test_login_flow.py::test_max_respawns_matches_source, keep this in
lockstep with init: bumping init's constant fails those pins until this
value is updated too, so a respawn-limit change surfaces in exactly one
authoritative place and the behavioral ports (RespawnAccounting, the
login-manager unbounded-loop tests) adapt automatically.

NOTE: test_init_golden_trace.py replays REAL captured serial data (5
respawns before give-up) and asserts against MAX_RESPAWNS. Bumping the
limit therefore ALSO requires re-capturing that fixture -- the one
manual step beyond editing this constant.
"""

MAX_RESPAWNS = 5
