#!/usr/bin/python3
"""Install the built engine and native window in the current user's home."""
import datetime
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

repo = Path(__file__).resolve().parents[1]
if os.geteuid() == 0:
    raise SystemExit('Run as your desktop user, without sudo.')
try:
    import gi
    gi.require_version('Gtk', '3.0')
    gi.require_version('WebKit2', '4.1')
except (ImportError, ValueError):
    raise SystemExit('Install python3-gi and gir1.2-webkit2-4.1 through Mint Software Manager, then retry.')
binary = repo/'target/release/controller-studio'
if subprocess.check_output([str(binary), '--version'], text=True).strip() != '0.5.0':
    raise SystemExit('Build the 0.5.0 engine first: cargo build --release --locked')
home = Path.home()
# Desktop Exec uses its own quoting rules, not a shell.
engine = str(home/'.local/bin/controller-studio')
quoted = '"' + engine.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$').replace('%', '%%') + '"'
desktop = f'''[Desktop Entry]
Type=Application
Name=Controller Studio
Comment=Plug in a controller and make the buttons yours
Exec={quoted}
Icon=input-gaming
Terminal=false
Categories=Game;Utility;
StartupNotify=true
StartupWMClass=ControllerStudio
'''
files = {
    '.local/bin/controller-studio': (binary.read_bytes(), 0o755),
    '.local/bin/controller-studio-native.py': ((repo/'native/app.py').read_bytes(), 0o755),
    '.local/share/applications/controller-studio.desktop': (desktop.encode(), 0o644),
}
backup = home/'.local/state/controller-studio/desktop-backups'/datetime.datetime.now().strftime('%Y%m%d-%H%M%S-%f')
# Check every target before replacing anything. Never follow an existing link.
for rel in files:
    target = home/rel
    for part in [target, *target.parents]:
        if part == home.parent:
            break
        if part.is_symlink():
            raise SystemExit('Refusing symbolic link: ' + str(part))
    if target.exists() and (not target.is_file() or target.stat().st_uid != os.getuid()):
        raise SystemExit('Unexpected file or owner: ' + str(target))
backup.mkdir(parents=True, mode=0o700)
for rel in files:
    target = home/rel
    if target.exists():
        saved = backup/rel
        saved.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(target, saved)
def atomic(target, data, mode):
    target.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix='.studio-', dir=target.parent)
    try:
        with os.fdopen(fd, 'wb') as output:
            os.fchmod(output.fileno(), mode)
            output.write(data)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)

written = []
try:
    for rel, (data, mode) in files.items():
        atomic(home/rel, data, mode)
        written.append(rel)
except OSError:
    for rel in reversed(written):
        saved = backup/rel
        if saved.exists():
            atomic(home/rel, saved.read_bytes(), saved.stat().st_mode & 0o777)
        else:
            (home/rel).unlink()
    raise
print('Installed Controller Studio. Close the old helper before opening the new app. Backups: ' + str(backup))
