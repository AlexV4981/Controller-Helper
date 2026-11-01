"""Exercise the optional wrapper with fake programs only, outside Flatpak."""
import hashlib,os,pathlib,subprocess,tempfile
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-wrapper-') as tmp:
 home=pathlib.Path(tmp);root=home/'.var/app/net.kuribo64.melonDS/data/controller-studio-integration';root.mkdir(parents=True);(home/'.local/state/controller-studio').mkdir(parents=True)
 fake=root/'melonDS';stock=home/'stock';stock.write_text('#!/bin/sh\nprintf "STOCK:%s\\n" "$@"\n');stock.chmod(0o755)
 wrapper=home/'wrapper'
 def prepare(help_status=0):
  fake.write_text('#!/bin/sh\nif [ "$1" = --help ]; then exit '+str(help_status)+'; fi\nprintf "PATCHED:%s\\n" "$@"\n');fake.chmod(0o755)
  text=(repo/'scripts/melonds-runtime-wrapper.in').read_text().replace('@BINARY_SHA256@',hashlib.sha256(fake.read_bytes()).hexdigest()).replace('/app/bin/melonDS',str(stock));wrapper.write_text(text);wrapper.chmod(0o755)
 def run():return subprocess.run([str(wrapper),'a game.nds','--fullscreen'],env=dict(os.environ,HOME=str(home)),capture_output=True,text=True,check=True).stdout
 prepare();assert run()=='PATCHED:a game.nds\nPATCHED:--fullscreen\n'
 fake.write_text(fake.read_text()+'# changed\n');assert run().startswith('STOCK:')
 prepare();(root/'disabled').touch();assert run().startswith('STOCK:');(root/'disabled').unlink()
 prepare(1);assert run().startswith('STOCK:')
 fake.unlink();assert run().startswith('STOCK:')
 print('Optional runtime wrapper: exact arguments, digest mismatch, disabled, incompatible and missing binary checks passed')
