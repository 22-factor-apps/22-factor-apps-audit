from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).parents[1] / "app_surface_parity.py"
SPEC = importlib.util.spec_from_file_location("app_surface_parity", MODULE_PATH)
assert SPEC and SPEC.loader
parity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(parity)


class AppSurfaceParityTests(unittest.TestCase):
    def test_canonical_vocabulary_is_valid_and_stable(self) -> None:
        parity.validate_feature_vocabulary()
        self.assertEqual(
            parity.contract_digest(),
            "487bda243becc322e372e4a8e81f322f4f1cb6c4d0b23c2b54f5621fb9f6e032",
        )

    def test_pair_names_are_enforced(self) -> None:
        parity.validate_pair("flutter", "acme/acme-flutter", "acme/acme-desktop-app.rs")
        parity.validate_pair("rust", "acme/acme-desktop-app.rs", "acme/acme-flutter")
        with self.assertRaises(parity.ContractError):
            parity.validate_pair("flutter", "acme/acme-flutter", "other/acme-desktop-app.rs")

    def test_desktop_only_feature_requires_mobile_fallback(self) -> None:
        with self.assertRaises(parity.ContractError):
            parity.validate_feature_vocabulary(
                ({"id": "desktop.test", "required": ["flutter.desktop", "rust.desktop"]},)
            )

    def test_flutter_platform_checks_are_rejected_from_domain_layer(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "pubspec.yaml").write_text("name: fixture\n", encoding="utf-8")
            domain = root / "lib/domain"
            domain.mkdir(parents=True)
            (domain / "bad.dart").write_text(
                "final desktop = Platform.isMacOS;\n", encoding="utf-8"
            )
            with self.assertRaises(parity.ContractError):
                parity.validate_repository(
                    root,
                    "flutter",
                    "fixture/fixture-flutter",
                    "fixture/fixture-desktop-app.rs",
                )

    def test_adapter_layer_is_not_scanned_as_portable_core(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "pubspec.yaml").write_text("name: fixture\n", encoding="utf-8")
            adapter = root / "lib/adapters"
            adapter.mkdir(parents=True)
            (adapter / "desktop.dart").write_text(
                "final desktop = Platform.isMacOS;\n", encoding="utf-8"
            )
            receipt = parity.validate_repository(
                root,
                "flutter",
                "fixture/fixture-flutter",
                "fixture/fixture-desktop-app.rs",
            )
            self.assertEqual(receipt["status"], "pass")


if __name__ == "__main__":
    unittest.main()
