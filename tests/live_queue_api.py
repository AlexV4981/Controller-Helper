"""Fault injection in a disposable home; never starts a real emulator."""
import ctypes,http.cookiejar,json,os,pathlib,subprocess,tempfile,time,urllib.request,urllib.error
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-live-',dir=repo/'test-output') as tmp:
 home=pathlib.Path(tmp);config=home/'.var/app/net.kuribo64.melonDS/config/melonDS/melonDS.toml';config.parent.mkdir(parents=True);original='[Instance0]\nJoystickID=-1\n[Instance0.Keyboard]\nA=88\n';config.write_text(original)
 dol=home/'.var/app/org.DolphinEmu.dolphin-emu/config/dolphin-emu';dol.mkdir(parents=True);(dol/'WiimoteNew.ini').write_text('[Wiimote1]\nSource=0\n')
 fake=subprocess.Popen(['python3','-c','import ctypes,time;ctypes.CDLL(None).prctl(15,b"dolphin-emu",0,0,0);print("ready",flush=True);time.sleep(120)'],stdout=subprocess.PIPE,text=True);assert fake.stdout.readline().strip()=='ready'
 base='http://127.0.0.1:17649';process=None
 def start():
  global process,client
  process=subprocess.Popen([str(repo/'target/release/controller-studio'),'--background','--port','17649'],env=dict(os.environ,HOME=str(home)),stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,text=True)
  client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
  for _ in range(50):
   try:client.open(base,timeout=1).read();return
   except urllib.error.URLError:time.sleep(.1)
  raise RuntimeError('server did not start')
 def req(path,body=None):
  r=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers={'Origin':base,'Content-Type':'application/json'})
  try:
   with client.open(r,timeout=5) as f:return f.status,json.load(f)
  except urllib.error.HTTPError as e:return e.code,json.load(e)
 def stop():
  req('/api/quit',{});process.wait(timeout=5)
 try:
  start();s=req('/api/state')[1];assert s['controllers'],'USB controller needed for this integration test'
  c=s['controllers'][0];payload=dict(key=c['device']['key'],system='wii-sideways',mode='labels',player=c['preference']['player'],bindings=c['defaults']['wii-sideways'],auto_apply=False,game='SMNE01')
  # Shared policy is explicitly disabled before queuing the game edit.
  shared={**payload,'game':None};status,result=req('/api/save',shared);assert status==200 and result['queued'],result
  status,result=req('/api/save',payload);assert status==200 and result['queued'],result
  assert config.read_text()==original
  assert req('/api/state')[1]['apply_phase']=='waiting'
  stop();start();s=req('/api/state')[1];assert not s['auto_apply'] and s['apply_phase']=='waiting';assert config.read_text()==original
  fake.terminate();fake.wait(timeout=3)
  for _ in range(80):
   s=req('/api/state')[1]
   if not s['pending'] and s['apply_phase']=='idle':break
   time.sleep(.1)
  else:raise AssertionError(s)
  assert 'JoystickID = ' in config.read_text()
  # Force a real configuration error and ensure it is not reported as queued.
  config.write_text('invalid = [')
  status,result=req('/api/save',payload);assert status==400 and 'Invalid melonDS TOML' in result['error'],result
  print(json.dumps({'idle_external_Dolphin_blocks_writes':True,'queued_request_survives_restart_with_auto_off':True,'applies_after_full_exit':True,'file_error_is_not_queued_success':True}))
 finally:
  if process and process.poll() is None:
   try:stop()
   except Exception:process.terminate();process.wait(timeout=4)
  if fake.poll() is None:fake.terminate();fake.wait(timeout=3)
