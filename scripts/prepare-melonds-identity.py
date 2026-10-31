#!/usr/bin/env python3
"""Patch a separate, exact melonDS 1.1 source checkout. Does not build or install it."""
import argparse, hashlib, json, pathlib, subprocess, tempfile

repo = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=pathlib.Path)
args = parser.parse_args()
source = args.source.resolve(strict=True)
receipt = json.loads((repo/'integrations/melonds-1.1-controller-id.json').read_text())
originals = {}
for name, checks in receipt['files'].items():
    path = source/'src/frontend/qt_sdl'/name
    if path.is_symlink() or not path.resolve().is_relative_to(source):
        raise SystemExit(f'Unsafe source path: {path}')
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != checks['before_sha256']:
        raise SystemExit(f'{name} differs from upstream 1.1 (or is already patched); nothing changed')
    originals[path] = data
patch = repo/'integrations/melonds-1.1-controller-id.patch'
# Validate and stage every output before touching the source checkout.
with tempfile.TemporaryDirectory(prefix='melonds-identity-') as tmp:
    staged = pathlib.Path(tmp)
    for path, data in originals.items():
        dest = staged/path.relative_to(source)
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data)
    subprocess.run(['patch','--batch','--fuzz=0','-p1','-i',str(patch)],cwd=staged,check=True)
    updates = {}
    for name, checks in receipt['files'].items():
        path = source/'src/frontend/qt_sdl'/name
        data = (staged/path.relative_to(source)).read_bytes()
        if hashlib.sha256(data).hexdigest() != checks['after_sha256']:
            raise SystemExit('Patched output failed verification; nothing changed')
        updates[path] = data
    try:
        for path, data in updates.items():
            if path.read_bytes() != originals[path]:
                raise RuntimeError('Source changed during preparation')
            path.write_bytes(data)
    except BaseException:
        for path, data in originals.items():
            path.write_bytes(data)
        raise
print('Prepared exact melonDS 1.1 sources. Build and native verification are still required; no emulator was installed.')
