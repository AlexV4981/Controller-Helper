"""Real-engine desktop lifecycle with an isolated home and no physical input writes."""
import http.cookiejar,importlib.util,json,pathlib,socket,tempfile,urllib.request
repo=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('studio_native',repo/'native/app.py');app=importlib.util.module_from_spec(spec);spec.loader.exec_module(app)
with tempfile.TemporaryDirectory(prefix='studio-engine-') as tmp:
 with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
 connection=app.Connection(repo/'target/release/controller-studio',tmp,port)
 try:
  connection.ensure_ready();first=connection.process
  assert first and first.poll() is None
  connection.ensure_ready();assert connection.process is first
  client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()),urllib.request.ProxyHandler({}))
  def stop():
   client.open(connection.origin).read()
   request=urllib.request.Request(connection.origin+'/api/quit',data=b'{}',headers={'Origin':connection.origin,'Content-Type':'application/json'})
   assert json.load(client.open(request))['ok'];connection.process.wait(timeout=5)
  state=json.load(client.open(connection.origin+'/health'));assert connection.matches(state)
  stop();connection.ensure_ready();assert connection.process.pid!=first.pid
  stop();print('Real native-engine startup, reuse, authenticated stop and restart passed')
 finally:
  if connection.process and connection.process.poll() is None:
   connection.process.terminate();connection.process.wait(timeout=5)
