"""Local integration checks. Refuses to mutate a real home or non-preview server."""
import http.cookiejar, json, urllib.request, urllib.error
from pathlib import Path
BASE='http://127.0.0.1:17645'
jar=http.cookiejar.CookieJar();client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
client.open(BASE,timeout=5).read()
def request(path,body=None,origin=BASE,host=None,anonymous=False,content_type='application/json'):
 headers={}
 if body is not None:
  headers['Content-Type']=content_type
  if origin is not None:headers['Origin']=origin
 if host:headers['Host']=host
 req=urllib.request.Request(BASE+path,data=None if body is None else json.dumps(body).encode(),headers=headers,method='GET' if body is None else 'POST')
 try:
  r=(urllib.request.urlopen(req,timeout=5) if anonymous else client.open(req,timeout=5));return r.status,json.loads(r.read())
 except urllib.error.HTTPError as e:return e.code,json.loads(e.read())
code,state=request('/api/state');assert code==200 and state['preview'], 'Tests require an isolated preview home'
assert len(state['controllers'])==2, 'Connect both Xbox pads for this hardware integration pass'
checks={}
checks['anonymous_state_denied']=request('/api/state',anonymous=True)[0]==403
checks['foreign_origin_denied']=request('/api/defaults',{},origin='https://unrelated.invalid')[0]==403
checks['missing_mutation_origin_denied']=request('/api/defaults',{},origin=None)[0]==403
checks['rebinding_host_denied']=request('/api/state',host='unrelated.invalid')[0]==403
c=state['controllers'][0];payload=dict(key=c['device']['key'],system='ds',mode='labels',player=c['preference']['player'],bindings=dict(c['mappings']['ds']),auto_apply=True)
payload['bindings']['A']='../../not-a-controller\n[Injected]'
checks['unsafe_binding_denied']=request('/api/save',payload)[0]==400
checks['non_json_body_denied']=request('/api/defaults',{},content_type='text/plain')[0]==400
checks['unknown_endpoint_denied']=request('/api/run-command',{'command':'unused'})[0]==404
assert all(checks.values()), checks
out=Path(__file__).resolve().parents[1]/'test-output/api-security.json';out.write_text(json.dumps({'checks':checks,'physical_controllers_detected':len(state['controllers'])},indent=2)+'\n');print(json.dumps(checks))
