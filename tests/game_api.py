"""Adversarial API checks against an explicitly isolated preview home."""
import http.cookiejar,urllib.request,urllib.error,json,copy
base='http://127.0.0.1:17645';jar=http.cookiejar.CookieJar();client=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar));client.open(base).read()
def request(path,body=None):
 r=urllib.request.Request(base+path,data=None if body is None else json.dumps(body).encode(),headers={'Origin':base,'Content-Type':'application/json'})
 try:
  with client.open(r,timeout=5) as f:return f.status,json.load(f)
 except urllib.error.HTTPError as e:return e.code,json.load(e)
s=request('/api/state')[1];assert s['preview'];c=s['controllers'][0];key=c['device']['key'];old=copy.deepcopy(c['mappings']);oldmode=c['preference']['mode'];oldauto=s['auto_apply'];oldstyle=s['wii_style']
payload=dict(key=key,system='wii-classic',mode='positions',player=c['preference']['player'],bindings=copy.deepcopy(c['game_layouts']['RSBE01']['bindings']),auto_apply=not oldauto,game='RSBE01');payload['bindings']['A']='north'
assert request('/api/save',payload)[0]==200
s=request('/api/state')[1];c=next(x for x in s['controllers'] if x['device']['key']==key)
assert c['game_layouts']['RSBE01']['bindings']['A']=='north';assert c['mappings']==old;assert c['preference']['mode']==oldmode;assert s['auto_apply']==oldauto and s['wii_style']==oldstyle
for invalid in [dict(payload,game='../../etc/passwd'),dict(payload,system='ds'),dict(payload,player=4)]:assert request('/api/save',invalid)[0]==400
assert request('/api/launch',{'game':'ASME','path':'/etc/passwd'})[0]==400
assert request('/api/launch',{'game':'ASME'})[0]==400
assert request('/api/save',dict(payload,inherit=True))[0]==200
c=next(x for x in request('/api/state')[1]['controllers'] if x['device']['key']==key)
assert not c['game_layouts']['RSBE01']['custom'];assert c['game_layouts']['RSBE01']['bindings']==old['wii-classic']
print(json.dumps({'game_isolation':True,'mode_and_roles_preserved':True,'invalid_scopes_rejected':True,'preview_and_path_launch_rejected':True,'inheritance_restored':True}))
