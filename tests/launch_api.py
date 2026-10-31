"""Launch lifecycle fault injection. Uses a temporary HOME, fake ROM and fake Flatpak only."""
import os,sys,json,time,tempfile,pathlib,subprocess,urllib.request,urllib.error,http.cookiejar
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-launch-',dir=repo/'test-output') as tmp:
 home=pathlib.Path(tmp);fakebin=home/'bin';fakebin.mkdir();log=home/'command.json'
 fake=fakebin/'flatpak';fake.write_text('#!/usr/bin/python3\nimport json,os,sys,time\nopen(os.environ["TEST_COMMAND_LOG"],"w").write(json.dumps(sys.argv[1:]))\ntime.sleep(5)\n');fake.chmod(0o755)
 catalog=json.loads((repo/'src/games.json').read_text());game=next(g for g in catalog if g['id']=='ASME');rom=home/'Games/Nintendo/DS'/game['filename'];rom.parent.mkdir(parents=True);header=bytearray(96);header[12:16]=b'ASME';rom.write_bytes(header)
 config=home/'.var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml';config.parent.mkdir(parents=True);config.write_text('[Emu]\nConsoleType=0\n[Instance0]\nSaveFilePath=""\n[Instance0.Keyboard]\nA=88\n')
 env=dict(os.environ,HOME=str(home),PATH=str(fakebin)+':'+os.environ['PATH'],TEST_COMMAND_LOG=str(log))
 process=subprocess.Popen([str(repo/'target/release/controller-studio'),'--background','--port','17647'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
 base='http://127.0.0.1:17647';client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
 def req(path,body=None):
  r=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers={'Origin':base,'Content-Type':'application/json'})
  try:
   with client.open(r,timeout=4) as f:return f.status,json.load(f)
  except urllib.error.HTTPError as e:return e.code,json.load(e)
 try:
  for _ in range(40):
   try:client.open(base,timeout=1).read();break
   except urllib.error.URLError:time.sleep(.1)
  state=req('/api/state')[1];assert not state['preview']
  assert req('/api/launch',{'game':'ASME'})[0]==200
  assert req('/api/launch',{'game':'ASME'})[0]==400
  assert req('/api/restore',{})[0]==400
  if state['controllers']:
   c=state['controllers'][0];payload=dict(key=c['device']['key'],system='ds',mode='labels',player=c['preference']['player'],bindings=c['mappings']['ds'],auto_apply=False)
   result=req('/api/save',payload);assert result[0]==200 and result[1]['queued']
  deadline=time.monotonic()+10
  while time.monotonic()<deadline:
   latest=req('/api/state')[1]
   if latest['launch_status'].startswith('Game session closed') and (not state['controllers'] or not latest['pending']):break
   time.sleep(.15)
  else:raise AssertionError('manual queued save did not finish after startup lease closed')
  args=json.loads(log.read_text());assert '--fullscreen' in args;assert str(rom) in args;assert args[1:3] == ['--command=env', 'net.kuribo64.melonDS'];assert args[3].startswith('XDG_CONFIG_HOME=') and '/controller-studio-games/ASME' in args[3];assert args[4] == 'melonDS'
  print(json.dumps({'duplicate_launch_blocked':True,'startup_restore_blocked':True,'startup_save_queued':bool(state['controllers']),'manual_queue_applies_with_auto_off':bool(state['controllers']),'isolated_config_argument':True,'fullscreen_argument':True,'single_exact_ROM_argument':True}))
 finally:
  try:req('/api/quit',{})
  except Exception:pass
  try:process.wait(timeout=4)
  except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=4)
