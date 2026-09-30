#!/usr/bin/env python3
"""Install the matching prebuilt binding tool after verifying its checksum."""
import argparse
import hashlib
import io
import os
import platform
import re
import tarfile
import time
import tomllib
import urllib.request
from pathlib import Path


def download(url):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(url, timeout=30) as response:
                content = response.read(32 * 1024 * 1024 + 1)
            if len(content) > 32 * 1024 * 1024:
                raise ValueError("binding tool download exceeds 32 MiB")
            return content
        except OSError:
            if attempt == 3:
                raise
            time.sleep(2 ** attempt)
    raise RuntimeError("download did not finish")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    lock = tomllib.loads(Path("Cargo.lock").read_text(encoding="utf-8"))
    version = next(package["version"] for package in lock["package"] if package["name"] == "wasm-bindgen")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("invalid wasm-bindgen version in lockfile")
    machine = {"x86_64": "x86_64", "aarch64": "aarch64"}.get(platform.machine())
    if platform.system() != "Linux" or machine is None:
        raise ValueError("prebuilt installer supports Linux x86_64/aarch64; otherwise use cargo install")
    asset = f"wasm-bindgen-{version}-{machine}-unknown-linux-musl.tar.gz"
    base = f"https://github.com/rustwasm/wasm-bindgen/releases/download/{version}/"
    expected = download(base + asset + ".sha256sum").decode("ascii").split()[0]
    content = download(base + asset)
    if not re.fullmatch(r"[a-fA-F0-9]{64}", expected) or hashlib.sha256(content).hexdigest() != expected.lower():
        raise ValueError("binding tool checksum does not match")
    args.destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(content), mode="r:gz") as archive:
        matches = [member for member in archive.getmembers() if member.isfile() and Path(member.name).name == "wasm-bindgen"]
        if len(matches) != 1:
            raise ValueError("binding tool archive must contain exactly one executable")
        with archive.extractfile(matches[0]) as source:
            executable = args.destination / "wasm-bindgen"
            executable.write_bytes(source.read())
        executable.chmod(0o755)
    if "GITHUB_PATH" in os.environ:
        with open(os.environ["GITHUB_PATH"], "a", encoding="utf-8") as path_file:
            path_file.write(str(args.destination.resolve()) + "\n")
    print(f"Installed wasm-bindgen {version}")


if __name__ == "__main__":
    main()
