import importlib.util
from pathlib import Path
import struct
import unittest

spec = importlib.util.spec_from_file_location("verify_elf", Path(__file__).with_name("verify-elf.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def fixture(extra=None, dynamic=b"", machine=183, kind=2):
    phnum = 1 + (extra is not None)
    size = 256 + len(dynamic)
    data = bytearray(size)
    ident = b"\x7fELF\x02\x01\x01" + b"\x00" * 9
    struct.pack_into("<16sHHIQQQIHHHHHH", data, 0, ident, kind, machine, 1, 0x4000F0, 64, 0, 0, 64, 56, phnum, 0, 0, 0)
    struct.pack_into("<IIQQQQQQ", data, 64, 1, 5, 0, 0x400000, 0x400000, size, size, 4096)
    if extra is not None:
        struct.pack_into("<IIQQQQQQ", data, 120, extra, 4, 256, 0x400100, 0x400100, len(dynamic), len(dynamic), 8)
        data[256:] = dynamic
    return data


class StaticElfTests(unittest.TestCase):
    def test_static_exec_and_pie_without_needed_libraries(self):
        self.assertEqual(module.inspect_elf(fixture())["elfType"], "EXEC")
        self.assertEqual(module.inspect_elf(fixture(extra=2, dynamic=struct.pack("<qQ", 0, 0), kind=3))["elfType"], "static PIE")

    def test_runtime_loader_refuses(self):
        with self.assertRaisesRegex(ValueError, "PT_INTERP"):
            module.inspect_elf(fixture(extra=3, dynamic=b"/lib/ld-musl-aarch64.so.1\0"))

    def test_needed_library_refuses_even_without_interpreter(self):
        with self.assertRaisesRegex(ValueError, "DT_NEEDED"):
            module.inspect_elf(fixture(extra=2, dynamic=struct.pack("<qQqQ", 1, 5, 0, 0), kind=3))

    def test_wrong_arch_and_non_executable_refuse(self):
        for data in [fixture(machine=62), fixture(kind=1), b"!<arch>\n"]:
            with self.assertRaises(ValueError):
                module.inspect_elf(data)

    def test_truncation_and_nonterminating_dynamic_refuse(self):
        for data in [fixture()[:60], fixture()[:100], fixture()[:-1], fixture(extra=2, dynamic=struct.pack("<qQ", 5, 3))]:
            with self.assertRaises(ValueError):
                module.inspect_elf(data)


if __name__ == "__main__":
    unittest.main()
