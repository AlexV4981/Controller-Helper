"""Adversarial local tests; isolated preview only, never a real user's home."""
import http.cookiejar,json,socket,time,urllib.request,urllib.error,subprocess,sys
jar=http.cookiejar.CookieJar();c=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar));base='http://127.0.0.1:17645';c.open(base,timeout=4).read()
def req(path,data=None):
 r=urllib.request.Request(base+path,data=data,headers={'Origin':base,'Content-Type':'application/json'})
 try:
  response=c.open(r,timeout=5);return response.status,json.load(response)
 except urllib.error.HTTPError as e:return e.code,json.load(e)
assert req('/api/state')[1]['preview']
cookie='; '.join(f'{x.name}={x.value}' for x in jar)
checks={}
s=socket.create_connection(('127.0.0.1',17645),timeout=4)
try:
 s.sendall((f'POST /api/save HTTP/1.1\r\nHost: 127.0.0.1:17645\r\nOrigin: {base}\r\nCookie: {cookie}\r\nContent-Type: application/json\r\nContent-Length: 100000\r\n\r\n'+'{'+' '*8192).encode())
 start=time.monotonic();code,state=req('/api/state');checks['incomplete_body_does_not_hang_service']=code==200 and time.monotonic()-start<4
finally:s.close()
for body in [b'{',b'[]',b'null',b'{"__proto__":{}}',b'"'+b'x'*128001+b'"']:
 code,_=req('/api/save',body);assert code in [400,413],code
checks['malformed_and_oversized_bodies_rejected']=True
p=subprocess.Popen([sys.executable,'-c',"import ctypes,time;ctypes.CDLL(None).prctl(15,b'dolphin-emu',0,0,0);print('ready',flush=True);time.sleep(20)"],stdout=subprocess.PIPE,text=True)
try:
 assert p.stdout.readline().strip()=='ready'
 state=req('/api/state')[1];pad=state['controllers'][0];payload=dict(key=pad['device']['key'],system='ds',mode='labels',player=pad['preference']['player'],bindings=pad['mappings']['ds'],auto_apply=True)
 code,answer=req('/api/save',json.dumps(payload).encode());checks['running_emulator_blocks_writes']=code==200 and answer.get('queued') is True and 'Close dolphin-emu' in answer.get('message','')
finally:p.terminate();p.wait(timeout=4)
for _ in range(40):assert req('/api/state')[0]==200
checks['service_survives_request_burst']=True
assert all(checks.values()),checks
print(json.dumps(checks));open('test-output/stress-api.json','w').write(json.dumps(checks,indent=2)+'\n')
req('/api/quit',b'{}')
