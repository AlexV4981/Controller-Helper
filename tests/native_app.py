"""Desktop boundary tests; fake local HTTP services never touch user settings."""
import importlib.util,json,pathlib,tempfile,threading,unittest
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from unittest.mock import patch, Mock
ROOT=pathlib.Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('studio_native',ROOT/'native/app.py')
app=importlib.util.module_from_spec(spec);spec.loader.exec_module(app)
class NativeBoundary(unittest.TestCase):
 def test_native_and_engine_versions_match(self):
  import tomllib
  self.assertEqual(app.VERSION,tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'])

 def test_navigation_is_exact_origin_only(self):
  origin='http://127.0.0.1:17645'
  for uri in [origin,origin+'/',origin+'/index.html',origin+'/?x=1']:
   self.assertTrue(app.allowed_navigation(uri,origin),uri)
  for uri in ['https://example.com',origin+'.evil/',origin+'@evil/',origin+':80/','http://localhost:17645/','file:///etc/passwd','javascript:alert(1)','data:text/html,test','http://u:p@127.0.0.1:17645/','http://127.0.0.1:17646/','http://127.0.0.1:bad/']:
   self.assertFalse(app.allowed_navigation(uri,origin),uri)
 def test_health_identity_requires_app_version_and_same_home(self):
  c=app.Connection('/tmp/engine','/tmp/a home',17645)
  good={'app':'controller-studio','version':app.VERSION,'home':'/tmp/a home'}
  self.assertTrue(c.matches(good))
  for key,value in [('app','other'),('version','0.4.0'),('home','/tmp/other')]:
   self.assertFalse(c.matches(dict(good,**{key:value})))
  for value in [None,[],{},'hello']:
   self.assertFalse(c.matches(value))
 def test_port_validation(self):
  for port in [0,65536,-1]:
   with self.assertRaises(ValueError):app.Connection('/tmp/x','/tmp/home',port)
 def test_foreign_service_is_never_loaded_or_replaced(self):
  class Handler(BaseHTTPRequestHandler):
   def do_GET(self):
    self.send_response(200);self.end_headers();self.wfile.write(b'{"app":"something-else"}')
   def log_message(self,*args):pass
  server=ThreadingHTTPServer(('127.0.0.1',0),Handler);thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
  try:
   c=app.Connection('/missing','/tmp/home',server.server_port)
   with patch.object(app.subprocess,'Popen') as spawn:
    with self.assertRaisesRegex(RuntimeError,'already in use'):c.ensure_ready()
    spawn.assert_not_called()
  finally:server.shutdown();server.server_close();thread.join()
 def test_matching_engine_is_reused_without_another_process(self):
  c=app.Connection('/missing','/tmp/home',17645)
  with patch.object(c,'health',return_value={'app':'controller-studio','version':app.VERSION,'home':'/tmp/home'}),patch.object(app.subprocess,'Popen') as spawn:
   c.ensure_ready();spawn.assert_not_called()
 def test_missing_engine_is_actionable(self):
  c=app.Connection('/missing-engine','/tmp/home',17645)
  with patch.object(c,'health',return_value=None):
   with self.assertRaisesRegex(RuntimeError,'engine is missing'):c.ensure_ready()
 def test_retry_waits_for_owned_starting_engine(self):
  c=app.Connection('/missing','/tmp/home',17645);owned=Mock();owned.poll.return_value=None;c.process=owned
  good={'app':'controller-studio','version':app.VERSION,'home':'/tmp/home'}
  with patch.object(c,'health',side_effect=[None,good]),patch.object(app.subprocess,'Popen') as spawn:
   c.ensure_ready();spawn.assert_not_called();self.assertIs(c.process,owned)
 def test_health_never_follows_redirects(self):
  reached=[]
  class Target(BaseHTTPRequestHandler):
   def do_GET(self):
    reached.append(self.path);self.send_response(200);self.end_headers();self.wfile.write(b'{}')
   def log_message(self,*args):pass
  target=ThreadingHTTPServer(('127.0.0.1',0),Target)
  class Redirect(BaseHTTPRequestHandler):
   def do_GET(self):
    self.send_response(302);self.send_header('Location',f'http://127.0.0.1:{target.server_port}/outside');self.end_headers()
   def log_message(self,*args):pass
  source=ThreadingHTTPServer(('127.0.0.1',0),Redirect)
  threads=[threading.Thread(target=x.serve_forever,daemon=True) for x in [source,target]]
  for t in threads:t.start()
  try:
   c=app.Connection('/missing','/tmp/home',source.server_port);self.assertEqual(c.health(),{});self.assertEqual(reached,[])
  finally:
   for x in [source,target]:x.shutdown();x.server_close()
   for t in threads:t.join()
if __name__=='__main__':unittest.main()
