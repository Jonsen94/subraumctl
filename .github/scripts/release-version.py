"""Stamp Cargo's package and lockfile versions for a commit-based release."""

import re
import sys
import tomllib
from pathlib import Path

sha = sys.argv[1]
if not re.fullmatch(r"[0-9a-f]{40}", sha):
    sys.exit("Expected a full Git commit SHA")

manifest = Path("Cargo.toml")
lockfile = Path("Cargo.lock")
manifest_text = manifest.read_text()
lockfile_text = lockfile.read_text()
package = tomllib.loads(manifest_text)["package"]
base_version = package["version"]
if not re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", base_version):
    sys.exit("Cargo.toml must contain a stable major.minor.patch version")

version = f"{base_version}-g{sha[:12]}"
manifest_text, manifest_count = re.subn(
    rf'(?m)^version = "{re.escape(base_version)}"$',
    f'version = "{version}"',
    manifest_text,
    count=1,
)
lockfile_text, lockfile_count = re.subn(
    rf'(\[\[package\]\]\nname = "{re.escape(package["name"])}"\nversion = )'
    rf'"{re.escape(base_version)}"',
    rf'\g<1>"{version}"',
    lockfile_text,
    count=1,
)
if manifest_count != 1 or lockfile_count != 1:
    sys.exit("Could not locate the package version in Cargo.toml and Cargo.lock")

manifest.write_text(manifest_text)
lockfile.write_text(lockfile_text)
print(version)
