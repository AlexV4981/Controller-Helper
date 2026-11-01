"""Discovery, rename/removal, and isolated launch checks in a disposable HOME.
Synthetic ROM fixtures exercise plumbing only; they are not gameplay evidence.
"""
import hashlib,http.cookiejar,json,os,pathlib,subprocess,tempfile,time,urllib.request,urllib.error
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-discovery-') as tmp:
 home=pathlib.Path(tmp);root=home/'Games/Nintendo/DS';root.mkdir(parents=True)
 def rom(name,seed):
  data=bytearray([seed]*4096);data[:12]=b'HOMEBREW    ';data[12:16]=b'####';data[18]=0
  for offset,value in [(0x20,512),(0x2c,256),(0x30,768),(0x3c,256)]:data[offset:offset+4]=value.to_bytes(4,'little')
  for offset in [0x40,0x44,0x48,0x4c,0x50,0x54,0x58,0x5c,0x68]:data[offset:offset+4]=bytes(4)
  (root/name).write_bytes(data);return 'ds-'+hashlib.sha256(data).hexdigest()
 first=rom('one.nds',1);second=rom('two.nds',2)
 config=home/'.var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml';config.parent.mkdir(parents=True);config.write_text('[Emu]\nConsoleType=0\nExternalBIOSEnable=false\n[Instance0]\nSaveFilePath=""\n[Instance0.Keyboard]\nA=88\n')
 binary=home/'bin';binary.mkdir();log=home/'command.json';fake=binary/'flatpak';fake.write_text('#!/usr/bin/python3\nimport os,json,sys,time\nopen(os.environ["TEST_COMMAND_LOG"],"w").write(json.dumps(sys.argv[1:]))\ntime.sleep(1)\n');fake.chmod(0o755)
 env=dict(os.environ,HOME=str(home),PATH=str(binary)+':'+os.environ['PATH'],TEST_COMMAND_LOG=str(log))
 process=subprocess.Popen([str(repo/'target/release/controller-studio'),'--background','--port','17653'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,text=True)
 base='http://127.0.0.1:17653';client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
 def req(path,body=None):
  request=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers={'Origin':base,'Content-Type':'application/json'})
  try:
   with client.open(request,timeout=15) as f:return f.status,json.load(f)
  except urllib.error.HTTPError as e:return e.code,json.load(e)
 try:
  for _ in range(60):
   if process.poll() is not None:raise RuntimeError(process.stderr.read())
   try:client.open(base,timeout=1).read();break
   except urllib.error.URLError:time.sleep(.1)
  state=req('/api/state')[1];assert {first,second}<={g['id'] for g in state['games']}
  assert req('/api/launch',{'game':first,'path':'/etc/passwd'})[0]==400
  assert req('/api/launch',{'game':first})[0]==200
  for _ in range(50):
   if log.exists():break
   time.sleep(.1)
  args=json.loads(log.read_text());assert str(root/'one.nds') in args;assert any('controller-studio-games/'+first in a for a in args);assert '--fullscreen' in args
  for _ in range(60):
   state=req('/api/state')[1]
   if state['launch_status'].startswith('Game session closed'):break
   time.sleep(.1)
  (root/'one.nds').rename(root/'new name.nds');assert req('/api/library/refresh',{})[0]==200
  state=req('/api/state')[1];assert next(g for g in state['games'] if g['id']==first)['available']
  (root/'new name.nds').unlink();assert req('/api/library/refresh',{})[0]==200
  state=req('/api/state')[1];assert not next(g for g in state['games'] if g['id']==first)['available'];assert req('/api/launch',{'game':first})[0]==400
  assert req('/api/launch',{'game':'ds-'+'0'*64})[0]==400
  assert req('/api/quit',{})[0]==200;process.wait(timeout=5)
  print(json.dumps({'discovery':True,'exact_launch_arguments':True,'rename_identity':True,'missing_identity_retained':True,'unknown_launch_rejected':True,'synthetic_only':True}))
 finally:
  if process.poll() is None:process.terminate();process.wait(timeout=5)
