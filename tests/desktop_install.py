"""Install into a disposable home, preserving existing controls and rejecting links."""
import os,pathlib,subprocess,tempfile
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-install-') as tmp:
 home=pathlib.Path(tmp);pref=home/'.local/state/controller-studio/preferences.json';pref.parent.mkdir(parents=True);pref.write_text('{"sentinel":"keep roles"}')
 env=dict(os.environ,HOME=tmp)
 def install():return subprocess.run(['/usr/bin/python3',str(repo/'scripts/install-desktop.py')],env=env,capture_output=True,text=True,timeout=10)
 result=install();assert result.returncode==0,result.stderr+result.stdout
 binary=home/'.local/bin/controller-studio';assert binary.read_bytes()==(repo/'target/release/controller-studio').read_bytes()
 assert pref.read_text()=='{"sentinel":"keep roles"}'
 desktop=(home/'.local/share/applications/controller-studio.desktop').read_text();assert 'StartupWMClass=ControllerStudio' in desktop and 'firefox' not in desktop
 before=binary.read_bytes();result=install();assert result.returncode==0,result.stderr
 # A failed second replacement must restore the already-replaced engine.
 import runpy
 from unittest.mock import patch
 original_replace=os.replace;calls=[0]
 def fail_second(*args,**kwargs):
  calls[0]+=1
  if calls[0]==2:raise OSError('injected disk write failure')
  return original_replace(*args,**kwargs)
 with patch.dict(os.environ,{'HOME':tmp}),patch('os.replace',side_effect=fail_second):
  try:runpy.run_path(str(repo/'scripts/install-desktop.py'))
  except OSError as error:assert 'injected' in str(error)
  else:raise AssertionError('Injected failure was swallowed')
 assert binary.read_bytes()==before

 backups=list((home/'.local/state/controller-studio/desktop-backups').glob('*/.local/bin/controller-studio'));assert any(p.read_bytes()==before for p in backups)
 shell=home/'.local/bin/controller-studio-native.py';shell.unlink();other=home/'untouched';other.write_text('keep');shell.symlink_to(other)
 result=install();assert result.returncode!=0 and 'symbolic link' in result.stderr,result
 assert other.read_text()=='keep'
 print('Desktop install, backup, settings preservation and symlink refusal passed')
