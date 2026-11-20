from hashlib import sha256
from pathlib import Path
import unittest
from unittest.mock import patch

from . import w11_guest_observer_074 as driver


CONTEXT = "1" * 64
BEGIN = b"W11_OBSERVER_BEGIN " + CONTEXT.encode() + b"\r\n"
END = b"W11_OBSERVER_END " + CONTEXT.encode() + b"\r\n"
EXIT = b"W11_GUEST_EXIT=0\r\n"


class ObserverClockTests(unittest.TestCase):
    def test_existing_job_and_observer_windows_still_fit(self):
        self.assertEqual((driver.PROCESS_SECONDS, driver.OBSERVER_SECONDS, driver.JOB_SECONDS), (60, 150, 300))
        deadline = 300
        last = deadline - driver.VM_SECONDS - driver.UPLOAD_SECONDS
        driver.can_boot(deadline, last)
        with self.assertRaisesRegex(ValueError, "insufficient job time"):
            driver.can_boot(deadline, last + 0.001)

    def test_exact_order_scope_and_single_guest_exit(self):
        self.assertEqual(driver.serial_state(b"boot\n" + BEGIN + END + EXIT, CONTEXT), (2, True))
        for raw in (END + BEGIN, BEGIN + BEGIN, BEGIN + END + EXIT * 2,
                    BEGIN + END + b"W11_GUEST_EXIT=1\n",
                    BEGIN.replace(b"1", b"2") + END):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                driver.serial_state(raw, CONTEXT)

    def test_phase_edges_and_partial_lines(self):
        def step(state, raw, stamp, deadline=300):
            with patch.object(driver, "serial_read", return_value=raw), patch.object(driver.time, "monotonic", return_value=stamp):
                return driver.guest_phase(Path.cwd(), CONTEXT, deadline, 0, state)

        phases = []
        step(phases, BEGIN[:-2], 70)
        self.assertEqual(phases, [])
        step(phases, BEGIN, 95)
        step(phases, BEGIN + END, 250)
        step(phases, BEGIN + END + EXIT, 270)
        self.assertEqual(phases, [95, 250])
        for state, raw, stamp, deadline, error in (
            ([], b"", 95.01, 300, "boot exceeded"),
            ([95], BEGIN, 250.01, 300, "observation exceeded"),
            ([95, 250], BEGIN + END, 270.01, 300, "return or shutdown exceeded"),
            ([95, 250], BEGIN, 255, 300, "marks disappeared"),
            ([95, 250], BEGIN + END, 270, 269, "job deadline expired"),
        ):
            with self.subTest(error=error), self.assertRaisesRegex(ValueError, error):
                step(state, raw, stamp, deadline)

    def test_runtime_inventory_digest_is_independently_pinned(self):
        inventory = {"pinned-boot/vmlinuz": {"bytes": 2, "sha256": "b" * 64}}
        pin = sha256(driver.transfer.receipt.canonical(inventory)).hexdigest()
        with patch.object(driver, "runtime_inventory", return_value=inventory):
            self.assertEqual(driver.check_runtime(Path.cwd(), pin), inventory)
            with self.assertRaisesRegex(ValueError, "differs from its approval"):
                driver.check_runtime(Path.cwd(), "f" * 64)


if __name__ == "__main__":
    unittest.main()
