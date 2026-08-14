#!/usr/bin/env python3
"""Host-runnable pins for tests/validate_tcl.exp (the Tcl-syntax validator).

The validator runs `info complete` over each .exp harness so expect syntax
errors are caught in CI host-tests BEFORE the QEMU jobs boot. These tests
pin (a) the validator's behavior on valid and deliberately-broken files,
and (b) that the CI host-tests job actually runs it over every harness.

Run:  python3 tests/test_tcl_validator.py
"""
import io
import os
import shutil
import subprocess
import tempfile
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VALIDATOR = os.path.join(REPO_ROOT, "tests", "validate_tcl.exp")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")

def _read(p):
    with open(p, encoding="utf-8") as fh:
        return fh.read()


def _find_tclsh():
    for cand in ("tclsh", "tclsh8.6", "tclsh9.0"):
        exe = shutil.which(cand)
        if exe:
            return exe
    return None


class TclValidatorContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tclsh = _find_tclsh()
        cls.ci = _read(CI)

    def run_validator(self, *files):
        """Run the validator; return (rc, combined output)."""
        if not self.tclsh:
            self.skipTest("no tclsh available on this host")
        r = subprocess.run(
            [self.tclsh, VALIDATOR] + list(files),
            capture_output=True,
            text=True,
        )
        return r.returncode, (r.stdout or "") + (r.stderr or "")

    def test_validator_declares_itself(self):
        # The validator must exist and be a real tclsh script using
        # `info complete` - the Tcl-core syntax-completeness primitive.
        self.assertTrue(os.path.exists(VALIDATOR), "tests/validate_tcl.exp missing")
        v = _read(VALIDATOR)
        self.assertIn("info complete", v)
        self.assertIn("VALIDATE-TCL-PASS", v)
        self.assertIn("VALIDATE-TCL-FAIL", v)

    def test_all_harnesses_parse_complete(self):
        # Every .exp harness must parse as complete Tcl commands. The glob
        # mirrors the CI step exactly (tests/*.exp), so a NEW harness gets
        # local coverage automatically - no hardcoded list to forget.
        import glob

        paths = sorted(glob.glob(os.path.join(REPO_ROOT, "tests", "*.exp")))
        self.assertTrue(len(paths) >= 7, f"expected harnesses, found {paths}")
        rc, out = self.run_validator(*paths)
        self.assertEqual(rc, 0, out)
        self.assertIn("VALIDATE-TCL-PASS", out)

    def test_broken_harness_is_rejected(self):
        # A deliberately malformed harness (unterminated quote) must be
        # reported INCOMPLETE with a non-zero exit - the validator has real
        # teeth, not a vacuous pass.
        with tempfile.NamedTemporaryFile(
            "w", suffix=".exp", delete=False, encoding="utf-8"
        ) as fh:
            fh.write(
                '#!/usr/bin/expect -f\n'
                'set x [lindex $argv 0]\n'
                'expect {\n'
                '    {foo} {\n'
                '        send_user "unterminated\n'  # unterminated quote
                '    timeout { puts "x" }\n'
                '}\n'
            )
            broken = fh.name
        try:
            rc, out = self.run_validator(broken)
        finally:
            os.unlink(broken)
        self.assertEqual(rc, 1, out)
        self.assertIn("INCOMPLETE", out)
        self.assertIn("VALIDATE-TCL-FAIL", out)

    def test_missing_file_is_rejected(self):
        # A missing harness must fail loudly (exit 1), not be skipped.
        rc, out = self.run_validator(
            os.path.join(REPO_ROOT, "tests", "does_not_exist.exp")
        )
        self.assertEqual(rc, 1, out)
        self.assertIn("ERROR: cannot open", out)

    def test_ci_runs_validator_over_all_harnesses(self):
        # The host-tests job must invoke the validator over every .exp.
        # File order does NOT matter (host-tests is defined after the QEMU
        # jobs in the YAML; jobs run in dependency-parallel order), so the
        # pin is on the STEP's presence and its glob, not position.
        block = self.ci[self.ci.index("host-tests:"):]
        self.assertIn("validate_tcl.exp", block)
        self.assertIn("tests/*.exp", block)
        # The step must fail the job when the validator rejects a harness
        # (a bare `|| true` would silently swallow the syntax gate).
        self.assertIn("exit 1", block)
        # Execution order: every QEMU job must WAIT for host-tests (the
        # syntax gate) via needs:, or the boots would run in parallel with
        # the check and the gate would be vacuous.
        for job in ("integration", "gui-login", "ade-selftest", "gui-gate"):
            jidx = self.ci.index(f"  {job}:")
            self.assertIn(
                "needs: [build, host-tests]", self.ci[jidx:jidx + 160], job
            )

    def test_validator_self_parses(self):
        # The validator must pass its own check (it is itself Tcl).
        rc, out = self.run_validator(VALIDATOR)
        self.assertEqual(rc, 0, out)


if __name__ == "__main__":
    unittest.main()
