"""HTTP fault checks against a disposable home; no controllers or real configs needed."""
import http.cookiejar,json,pathlib,socket,subprocess,tempfile,time,urllib.error,urllib.request
repo=pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='studio-http-') as home:
 with socket.socket() as port_socket:port_socket.bind(('127.0.0.1',0));port=port_socket.getsockname()[1]
 base=f'http://127.0.0.1:{port}'
 process=subprocess.Popen([str(repo/'target/release/controller-studio'),'--background','--home',home,'--port',str(port)],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
 client=urllib.request.build_opener(urllib.request.ProxyHandler({}),urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
 def req(path,data=None,origin=base,anonymous=False,host=None,kind='application/json'):
  headers={'Content-Type':kind}
  if origin is not None:headers['Origin']=origin
  if host:headers['Host']=host
  request=urllib.request.Request(base+path,data=data,headers=headers)
  try:
   opener=urllib.request.build_opener(urllib.request.ProxyHandler({})) if anonymous else client
   with opener.open(request,timeout=5) as response:return response.status,json.load(response)
  except urllib.error.HTTPError as error:return error.code,json.load(error)
 try:
  for _ in range(100):
   try:client.open(base,timeout=1).read();break
   except OSError:time.sleep(.05)
  assert req('/api/state')[1]['preview']
  assert req('/api/state',anonymous=True)[0]==403
  assert req('/api/defaults',b'{}',origin='https://unrelated.invalid')[0]==403
  assert req('/api/defaults',b'{}',origin=None)[0]==403
  assert req('/api/state',host='unrelated.invalid')[0]==403
  assert req('/api/defaults',b'{}',kind='text/plain')[0]==400
  assert req('/api/run-command',b'{"command":"unused"}')[0]==404
  for body in [b'{',b'[]',b'null',b'{"__proto__":{}}',b'"'+b'x'*128001+b'"']:
   assert req('/api/save',body)[0] in [400,413]
  with socket.create_connection(('127.0.0.1',port),timeout=4) as slow:
   cookie='; '.join(f'{item.name}={item.value}' for handler in client.handlers if isinstance(handler,urllib.request.HTTPCookieProcessor) for item in handler.cookiejar)
   slow.sendall((f'POST /api/save HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {base}\r\nCookie: {cookie}\r\nContent-Type: application/json\r\nContent-Length: 100000\r\n\r\n'+'{'+' '*8192).encode())
   started=time.monotonic();assert req('/api/state')[0]==200;assert time.monotonic()-started<4
  for _ in range(30):assert req('/api/state')[0]==200
  print('HTTP authentication, origin, host, malformed/oversized/slow requests and burst checks passed')
 finally:
  if process.poll() is None:
   try:req('/api/quit',b'{}');process.wait(timeout=5)
   except Exception:process.terminate();process.wait(timeout=5)
