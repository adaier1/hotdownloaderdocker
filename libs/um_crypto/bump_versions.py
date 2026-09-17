#!/usr/bin/env python3

import argparse
import glob
import re
from typing import Generator


def parse_version(text: str) -> str:
    re_version = re.compile(r'^\d+\.\d+\.\d+(-[-a-zA-Z\d]*)?$')
    if not re_version.match(text):
        raise argparse.ArgumentTypeError(f"Invalid version format: '{text}'")
    return text

def iter_cargo_toml_paths() -> Generator[str, None, None]:
    yield "um_audio/Cargo.toml"
    yield "um_wasm/Cargo.toml"
    yield "um_cli/Cargo.toml"
    for p in glob.iglob("um_crypto/*/Cargo.toml", recursive=False):
        yield p

def re_replace_file(path: str, regex: re.Pattern, replacement: str) -> None:
    print(f"Bumping {path}...")
    with open(path, "r", encoding='utf-8') as f:
        content = f.read()
    content = regex.sub(replacement, content, 1)
    with open(path, "w", encoding='utf-8', newline='\n') as f:
        f.write(content)

def main():
    parser = argparse.ArgumentParser (description="Bump versions")
    parser.add_argument("version", type=parse_version, help="Version to bump to")
    args = parser.parse_args()

    re_toml_version = re.compile(r'^\s*(version|"version"|\'version\')\s*=\s*.*$', re.MULTILINE)
    re_json_version = re.compile(r'^(\s*)"version"\s*:\s*".*?"\s*(,)?\s*$', re.MULTILINE)

    for path in iter_cargo_toml_paths():
        re_replace_file(path, re_toml_version, f'version = "{args.version}"')
    re_replace_file("um_wasm_loader/package.json", re_json_version, fr'\1"version": "{args.version}"\2')



if __name__ == "__main__":
    main()

