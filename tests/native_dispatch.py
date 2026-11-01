"""Exercise the real Rust entrypoint with a harmless native-window stand-in."""
import json,os,pathlib,shutil,subprocess,tempfile
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-desktop-') as tmp:
 root=pathlib.Path(tmp);engine=root/'controller-studio';shutil.copyfile(repo/'target/release/controller-studio',engine);engine.chmod(0o755)
 shell=root/'controller-studio-native.py';receipt=root/'args.json'
 shell.write_text('import pathlib,sys,json\npathlib.Path('+repr(str(receipt))+').write_text(json.dumps(sys.argv[1:]))\n')
 home=root/'a home';home.mkdir()
 run=subprocess.run([str(engine),'--home',str(home),'--port','19764'],capture_output=True,text=True,timeout=10)
 assert run.returncode==0,run.stderr
 assert json.loads(receipt.read_text())==['--backend',str(engine),'--home',str(home),'--port','19764']
 receipt.unlink();shell.unlink()
 run=subprocess.run([str(engine),'--home',str(home)],capture_output=True,text=True,timeout=10)
 assert run.returncode!=0 and 'native window is missing' in run.stderr,run
 assert not receipt.exists()
 print('Native default dispatch and missing-window failure passed; no browser command used')
