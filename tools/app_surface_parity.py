#!/usr/bin/env python3
"""Validate the ORES Flutter/Rust desktop parity contract.

This validator is intentionally dependency-free so it can be vendored or fetched
by an exact commit digest from public CI. It never needs a GitHub credential.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
from typing import Iterable

SCHEMA = "ores.app-surface-parity/v1"
POLICY_REFERENCE = "https://github.com/ORESoftware/my-ai/blob/main/AGENTS.md"
SURFACES = frozenset({"flutter.mobile", "flutter.desktop", "rust.desktop"})
FEATURES: tuple[dict[str, object], ...] = (
    {"id": "app.lifecycle", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "auth.session", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "navigation.routes", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "settings.preferences", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "diagnostics.redacted", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "offline.queue", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "sync.background", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "deep_links.https", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "accessibility.input", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {"id": "notifications.user_visible", "required": ["flutter.mobile", "flutter.desktop", "rust.desktop"]},
    {
        "id": "desktop.window.lifecycle",
        "required": ["flutter.desktop", "rust.desktop"],
        "mobile_fallback": "app.lifecycle",
    },
    {
        "id": "desktop.tray.lifecycle",
        "required": ["flutter.desktop", "rust.desktop"],
        "mobile_fallback": "notifications.user_visible",
    },
    {
        "id": "desktop.file_picker",
        "required": ["flutter.desktop", "rust.desktop"],
        "mobile_fallback": "mobile.document_picker",
    },
    {
        "id": "desktop.keyboard_shortcuts",
        "required": ["flutter.desktop", "rust.desktop"],
        "mobile_fallback": "accessibility.input",
    },
)

PORTABLE_ROOTS = {
    "flutter": ("lib/domain", "lib/application"),
    "rust": ("src/domain", "src/application"),
}
FORBIDDEN_PORTABLE_TOKENS = {
    "flutter": (
        "dart:io",
        "Platform.is",
        "defaultTargetPlatform",
        "kIsWeb",
        "MethodChannel",
    ),
    "rust": (
        "tauri",
        "wry",
        "web-view",
        "webview",
    ),
}
LANGUAGE_MARKERS = {"flutter": "pubspec.yaml", "rust": "Cargo.toml"}


class ContractError(ValueError):
    """Raised when a parity contract is invalid."""


def canonical_feature_json() -> bytes:
    return json.dumps(FEATURES, sort_keys=True, separators=(",", ":")).encode("utf-8")


def contract_digest() -> str:
    return hashlib.sha256(canonical_feature_json()).hexdigest()


def _org(repository: str) -> str:
    parts = repository.split("/", 1)
    if len(parts) != 2 or not all(parts):
        raise ContractError(f"repository must be owner/name: {repository!r}")
    return parts[0]


def validate_pair(implementation: str, local_repository: str, peer_repository: str) -> None:
    if implementation not in {"flutter", "rust"}:
        raise ContractError(f"unknown implementation: {implementation!r}")

    if _org(local_repository).casefold() != _org(peer_repository).casefold():
        raise ContractError("paired repositories must remain organization-local")

    if implementation == "flutter":
        if not local_repository.casefold().endswith("-flutter"):
            raise ContractError("Flutter repository must match *-flutter")
        if not peer_repository.casefold().endswith("-desktop-app.rs"):
            raise ContractError("Flutter peer must match *-desktop-app.rs")
    else:
        if not local_repository.casefold().endswith("-desktop-app.rs"):
            raise ContractError("Rust repository must match *-desktop-app.rs")
        if not peer_repository.casefold().endswith("-flutter"):
            raise ContractError("Rust peer must match *-flutter")


def validate_feature_vocabulary(features: Iterable[dict[str, object]] = FEATURES) -> None:
    ids: list[str] = []
    covered: set[str] = set()

    for item in features:
        feature_id = item.get("id")
        required_raw = item.get("required")
        if not isinstance(feature_id, str) or not feature_id:
            raise ContractError("every feature needs a non-empty id")
        if not isinstance(required_raw, list) or not required_raw:
            raise ContractError(f"{feature_id}: required must be a non-empty list")
        required = set(required_raw)
        if not required <= SURFACES:
            raise ContractError(f"{feature_id}: unsupported surfaces {sorted(required - SURFACES)}")
        if "flutter.desktop" in required and "flutter.mobile" not in required:
            fallback = item.get("mobile_fallback")
            if not isinstance(fallback, str) or not fallback:
                raise ContractError(f"{feature_id}: desktop-only Flutter feature needs mobile_fallback")
        ids.append(feature_id)
        covered |= required

    if len(ids) != len(set(ids)):
        raise ContractError("feature ids must be unique")
    if covered != SURFACES:
        raise ContractError(f"feature vocabulary misses surfaces: {sorted(SURFACES - covered)}")


def scan_portable_layers(root: Path, implementation: str) -> list[str]:
    violations: list[str] = []
    for relative_root in PORTABLE_ROOTS[implementation]:
        portable_root = root / relative_root
        if not portable_root.exists():
            continue
        for path in sorted(portable_root.rglob("*")):
            if not path.is_file() or path.suffix not in {".dart", ".rs"}:
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            for token in FORBIDDEN_PORTABLE_TOKENS[implementation]:
                if token.casefold() in text.casefold():
                    violations.append(
                        f"{path.relative_to(root)}: platform token {token!r} belongs in an adapter"
                    )
    return violations


def validate_repository(
    root: Path,
    implementation: str,
    local_repository: str,
    peer_repository: str,
) -> dict[str, object]:
    validate_pair(implementation, local_repository, peer_repository)
    validate_feature_vocabulary()

    marker = root / LANGUAGE_MARKERS[implementation]
    if not marker.is_file():
        raise ContractError(f"missing language marker: {marker.relative_to(root)}")

    violations = scan_portable_layers(root, implementation)
    if violations:
        raise ContractError("\n".join(violations))

    return {
        "schema": SCHEMA,
        "policy_reference": POLICY_REFERENCE,
        "pair": _org(local_repository),
        "implementation": implementation,
        "local_repository": local_repository,
        "peer_repository": peer_repository,
        "feature_count": len(FEATURES),
        "feature_ids": [item["id"] for item in FEATURES],
        "contract_digest": contract_digest(),
        "portable_core_violations": 0,
        "status": "pass",
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--implementation", choices=("flutter", "rust"), required=True)
    parser.add_argument("--local-repository", required=True)
    parser.add_argument("--peer-repository", required=True)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--receipt-out", type=Path)
    return parser


def main() -> int:
    args = build_parser().parse_args()
    try:
        receipt = validate_repository(
            args.root.resolve(),
            args.implementation,
            args.local_repository,
            args.peer_repository,
        )
    except ContractError as exc:
        print(str(exc), file=os.sys.stderr)
        return 1

    rendered = json.dumps(receipt, indent=2, sort_keys=True) + "\n"
    print(rendered, end="")
    if args.receipt_out:
        args.receipt_out.parent.mkdir(parents=True, exist_ok=True)
        args.receipt_out.write_text(rendered, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
