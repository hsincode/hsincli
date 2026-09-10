"""Run native Hsin builds with upstream's verified sandbox-enabled V8."""

import os
from pathlib import Path
import subprocess
import sys


def main() -> int:
    repo = Path(__file__).resolve().parent.parent
    os.environ["CODEX_REPO_ROOT"] = str(repo)

    from codex_package.targets import TARGET_SPECS
    from codex_package.v8 import resolve_codex_v8_cargo_env

    rust_root = repo / "codex-rs"
    version = subprocess.check_output(["rustc", "-vV"], cwd=rust_root, text=True)
    host = next(
        line.removeprefix("host: ")
        for line in version.splitlines()
        if line.startswith("host: ")
    )
    target = os.environ.get("CARGO_BUILD_TARGET", host)
    env = {**os.environ, **resolve_codex_v8_cargo_env(TARGET_SPECS[target])}
    return subprocess.run(sys.argv[1:], cwd=rust_root, env=env).returncode


if __name__ == "__main__":
    sys.exit(main())
