"""Real ROM identity and in-emulator switch test on a private X server.
Requires a built patched melonDS, Xvfb, xdotool, and two user-supplied .nds/.zip files.
No original ROMs, configurations or saves are modified. No screenshots are captured.
"""
import argparse
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--tools-root',type=__import__('pathlib').Path,required=True)
parser.add_argument('--emulator',required=True)
parser.add_argument('roms',nargs=2,type=__import__('pathlib').Path)
args=parser.parse_args()
import os,pathlib,tempfile,subprocess,time,zipfile,hashlib,json,urllib.request,http.cookiejar,ctypes
ROOT=args.tools_root.resolve();REPO=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-real-ds-') as tmp:
 home=pathlib.Path(tmp);games=home/'Games/Nintendo/DS';games.mkdir(parents=True)
 ids=[];paths=[]
 for number,input_path in enumerate(args.roms):
  if input_path.suffix.lower()=='.zip':
   with zipfile.ZipFile(input_path) as z:
    members=[m for m in z.infolist() if m.filename.lower().endswith('.nds') and 512<=m.file_size<=536870912];assert len(members)==1
    data=z.read(members[0])
  else:
   assert input_path.suffix.lower()=='.nds' and 512<=input_path.stat().st_size<=536870912
   data=input_path.read_bytes()
  p=games/('game-'+str(number)+'.nds');p.write_bytes(data);paths.append(p);ids.append('ds-'+hashlib.sha256(data).hexdigest())
 cfg=home/'config/melonDS/melonDS.toml';cfg.parent.mkdir(parents=True);cfg.write_text('[Emu]\nConsoleType=0\nExternalBIOSEnable=false\nDirectBoot=true\n[3D]\nRenderer=0\n[Instance0]\nJoystickID=-1\n')
 runtime=home/'runtime';runtime.mkdir(mode=0o700)
 env=dict(os.environ,HOME=str(home),XDG_CONFIG_HOME=str(home/'config'),XDG_RUNTIME_DIR=str(runtime),LD_LIBRARY_PATH=str(ROOT/'usr/lib/x86_64-linux-gnu'),SDL_AUDIODRIVER='dummy',QT_QPA_PLATFORM='xcb',LIBGL_ALWAYS_SOFTWARE='1')
 display=subprocess.Popen([str(ROOT/'usr/bin/Xvfb'),'-displayfd','1','-screen','0','1024x768x24','-nolisten','tcp'],env=env,stdout=subprocess.PIPE,stderr=open(home/'xvfb.log','w'),text=True)
 number=display.stdout.readline().strip();assert number.isdecimal(),(home/'xvfb.log').read_text();env['DISPLAY']=':'+number
 def command(*args,check=True):return subprocess.run(args,env=env,capture_output=True,text=True,check=check)
 emulator=None;studio=None
 try:
  emulator=subprocess.Popen([args.emulator,str(paths[0])],env=env,stdout=open(home/'emu.log','w'),stderr=subprocess.STDOUT)
  def windows():
   return command(str(ROOT/'usr/bin/xdotool'),'search','--onlyvisible','--pid',str(emulator.pid),'--class','melonDS',check=False).stdout.split()
  def titles():
   return [command(str(ROOT/'usr/bin/xdotool'),'getwindowname',w,check=False).stdout.strip() for w in windows()]
  def wait_title(identity):
   deadline=time.monotonic()+25
   while time.monotonic()<deadline:
    if emulator.poll() is not None:raise RuntimeError((home/'emu.log').read_text())
    if any('[CS-ROM-v1:'+identity[3:]+']' in title for title in titles()):return
    time.sleep(.2)
   raise AssertionError({'titles':titles(),'log':(home/'emu.log').read_text()[-2000:]})
  wait_title(ids[0]);print('FIRST_ID_OK',flush=True)
  # A minimal test window-list publisher replaces a window manager on this private display.
  x=ctypes.CDLL('libX11.so.6');D=ctypes.c_void_p;W=ctypes.c_ulong
  x.XOpenDisplay.argtypes=[ctypes.c_char_p];x.XOpenDisplay.restype=D;d=x.XOpenDisplay(env['DISPLAY'].encode());assert d
  x.XDefaultRootWindow.argtypes=[D];x.XDefaultRootWindow.restype=W
  x.XInternAtom.argtypes=[D,ctypes.c_char_p,ctypes.c_int];x.XInternAtom.restype=W
  x.XChangeProperty.argtypes=[D,W,W,W,ctypes.c_int,ctypes.c_int,ctypes.c_void_p,ctypes.c_int]
  handles=windows();values=(W*len(handles))(*(int(w) for w in handles));x.XChangeProperty(d,x.XDefaultRootWindow(d),x.XInternAtom(d,b'_NET_CLIENT_LIST',0),33,32,0,values,len(handles))
  x.XFlush.argtypes=[D];x.XFlush(d);x.XCloseDisplay.argtypes=[D];x.XCloseDisplay(d)
  studio=subprocess.Popen([str(REPO/'target/release/controller-studio'),'--background','--port','17654'],env=env,stdout=subprocess.DEVNULL,stderr=open(home/'studio.log','w'))
  client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()));base='http://127.0.0.1:17654'
  for _ in range(100):
   try:client.open(base,timeout=1).read();break
   except OSError:time.sleep(.1)
  def state():return json.load(client.open(base+'/api/state',timeout=5))
  for _ in range(40):
   if state()['runtime']['game']==ids[0]:break
   time.sleep(.15)
  assert state()['runtime']['game']==ids[0],{'runtime':state()['runtime'],'issues':state().get('library_issues'),'games':[g for g in state()['games'] if g['id'].startswith('ds-')],'root':command('/usr/bin/xprop','-root','_NET_CLIENT_LIST',check=False).stdout,'props':[command('/usr/bin/xprop','-id',w,'_NET_WM_PID','WM_CLASS','WM_NAME',check=False).stdout for w in windows()]};print('STUDIO_FIRST_ID_OK',flush=True)
  w=windows()[0];xd=str(ROOT/'usr/bin/xdotool');command(xd,'windowfocus','--sync',w);command(xd,'key','--clearmodifiers','ctrl+o');time.sleep(.7);command(xd,'key','--clearmodifiers','ctrl+l');command(xd,'type','--clearmodifiers','--delay','1',str(paths[1]));command(xd,'key','Return')
  wait_title(ids[1]);print('SWITCH_ID_OK',flush=True)
  for _ in range(40):
   if state()['runtime']['game']==ids[1]:break
   time.sleep(.15)
  assert state()['runtime']['game']==ids[1],state()['runtime'];print('STUDIO_SWITCH_ID_OK',flush=True)
  # Quit through the application's standard menu shortcut, not a killed save operation.
  command(xd,'windowfocus',w);command(xd,'key','--clearmodifiers','ctrl+q')
  try:emulator.wait(timeout=8)
  except subprocess.TimeoutExpired:raise AssertionError('Emulator did not quit through Ctrl+Q')
  print(json.dumps({'two_user_supplied_roms':True,'loaded_byte_hash_matches':True,'in_emulator_switch':True,'studio_follows_exact_id':True,'emulator_exit':emulator.returncode,'private_virtual_display':True}),flush=True)
 except BaseException:
  print('DIAGNOSTICS',json.dumps({'titles':titles() if emulator else [],'emulator_log':(home/'emu.log').read_text()[-3000:] if (home/'emu.log').exists() else '', 'studio_log':(home/'studio.log').read_text()[-1000:] if (home/'studio.log').exists() else ''}),flush=True);raise
 finally:
  if studio and studio.poll() is None and 'client' in locals():
   try:client.open(urllib.request.Request(base+'/api/quit',data=b'{}',headers={'Origin':base,'Content-Type':'application/json'}),timeout=3).read()
   except (OSError,ValueError):pass
  for p in [studio,emulator,display]:
   if p and p.poll() is None:
    p.terminate()
    try:p.wait(timeout=3)
    except subprocess.TimeoutExpired:p.kill();p.wait(timeout=3)
