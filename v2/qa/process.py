#!/usr/bin/env python3
"""Actual independent SDK executables/OS pipes; owned TLS/selection/clipboard only."""
from pathlib import Path
import os,sys,json,uuid,select,subprocess,time,ssl,http.server,threading,queue,hashlib,email.parser,email.policy,atexit
R=Path(__file__).resolve().parents[1];Q=R/'.qa/pipes';Q.mkdir(parents=True,exist_ok=True)
children=[]
@atexit.register
def cleanup():
 for p in children:
  if p.poll()is None:p.terminate();p.wait(timeout=5)
checks=0
ASCII='ASCII\n```\n![remote](https://blocked.invalid/a)\n<img src="https://blocked.invalid/x" onerror="evil()">\n\u202e\n'
class TLS:
 def __init__(self):
  self.received=queue.Queue();self.modes=queue.Queue();self.gates=[];owner=self
  class Handler(http.server.BaseHTTPRequestHandler):
   def do_POST(self):
    assert self.path=='/image-to-ascii';assert self.headers.get('Authorization')is None;assert self.headers.get('Cookie')is None
    n=int(self.headers['Content-Length']);assert n<8_020_000
    b=self.rfile.read(n);message=email.parser.BytesParser(policy=email.policy.default).parsebytes(b'Content-Type: '+self.headers['Content-Type'].encode()+b'\r\nMIME-Version: 1.0\r\n\r\n'+b)
    parts=[{'name':p.get_param('name',header='content-disposition'),'filename':p.get_filename(),'mime':p.get_content_type(),'bytes':p.get_payload(decode=True).hex()}for p in message.iter_parts()]
    owner.received.put(parts)
    try:mode=owner.modes.get_nowait()
    except queue.Empty:mode=('text',None)
    if mode[0]=='hold':mode[1].wait(5)
    status=500 if mode[0]=='error'else 200
    body=(b'private server dump /not-for-ui'if status==500 else b'{"error":"not ASCII"}'if mode[0]=='object'else json.dumps(ASCII).encode()if mode[0]=='json'else ASCII.encode())
    try:
     self.send_response(status);self.send_header('Content-Type','text/plain');self.send_header('Content-Length',str(len(body)));self.send_header('Connection','close');self.end_headers();self.wfile.write(body)
    except (BrokenPipeError,ConnectionResetError,ssl.SSLError):pass
   def log_message(self,*_):pass
  self.server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler);self.server.daemon_threads=True
  ctx=ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER);ctx.load_cert_chain(R/'qa/fixtures/tls/server.pem',R/'qa/fixtures/tls/server.key');self.server.socket=ctx.wrap_socket(self.server.socket,server_side=True)
  self.thread=threading.Thread(target=self.server.serve_forever,daemon=True);self.thread.start();self.endpoint=f'https://localhost:{self.server.server_port}/image-to-ascii'
 def mode(self,name):
  gate=threading.Event()if name=='hold'else None
  if gate:self.gates.append(gate)
  self.modes.put((name,gate));return gate
 def next(self):return self.received.get(timeout=4)
 def close(self):
  for g in self.gates:g.set()
  self.server.shutdown();self.server.server_close();self.thread.join(3)
class Pipe:
 def __init__(self,binary,name,endpoint=None):
  binary=Path(binary).resolve();assert binary.is_relative_to(R)
  self.root=Q/name;self.root.mkdir(exist_ok=True)
  for d in ['home','temp','data','selected']:(self.root/d).mkdir(exist_ok=True)
  env={'PATH':os.environ['PATH'],'HOME':str(self.root/'home'),'USERPROFILE':str(self.root/'home'),'TMPDIR':str(self.root/'temp'),'TEMP':str(self.root/'temp'),'TMP':str(self.root/'temp')}
  argv=[str(binary)]
  if endpoint:env['OWNED_QA_ENDPOINT']=endpoint;argv+=['--ignored','--exact','tests::owned_pipe_worker','--nocapture']
  profile=self.root/'network.sb';profile.write_text('(version 1)\n(allow default)\n(deny network*)\n'+('(allow network-outbound (remote ip "localhost:*"))\n'if endpoint else''))
  sandbox=Path('/usr/bin/sandbox-exec');self.networkDenied=sandbox.exists()
  if sandbox.exists():argv=[str(sandbox),'-f',str(profile),*argv]
  self.p=subprocess.Popen(argv,cwd=R,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);children.append(self.p);self.buffer=b'';self.queue=[];self.reverse=[];self.harness=[]
  hello=self.read();assert hello['type']=='hello'and hello['id']=='com.tezbar.native.image-to-ascii'and hello['sdkVersion']=='0.1.1'
  package=binary.parent.parent if binary.parent.name=='bin'else R
  self.call('initialize',{'protocol':1,'extensionId':hello['id'],'platform':'macOS','packageDir':str(package),'dataDir':str(self.root/'data'),'tempDir':str(self.root/'temp'),'grantedCapabilities':['selection.read','clipboard.write']})
 def send(self,m):self.p.stdin.write(json.dumps(m,separators=(',',':')).encode()+b'\n');self.p.stdin.flush()
 def request(self,method,params):
  ident='h:'+str(uuid.uuid4());self.send({'type':'request','id':ident,'method':method,'params':params});return ident
 def read(self,timeout=5):
  deadline=time.monotonic()+timeout
  while True:
   while b'\n'not in self.buffer:
    remaining=deadline-time.monotonic();assert remaining>0,'read timeout';ready,_,_=select.select([self.p.stdout],[],[],remaining);assert ready,'read timeout';b=os.read(self.p.stdout.fileno(),65536);assert b,'EOF '+self.p.stderr.read().decode();self.buffer+=b
   line,self.buffer=self.buffer.split(b'\n',1)
   if not line.startswith(b'{'):self.harness.append(line.decode());continue
   return json.loads(line)
 def response(self,ident):
  while True:
   for n,m in enumerate(self.queue):
    if m.get('type')=='response'and m.get('id')==ident:return self.queue.pop(n)
   m=self.read()
   if m.get('type')=='response'and m.get('id')==ident:return m
   self.queue.append(m)
 def call(self,method,params):
  m=self.response(self.request(method,params));assert 'result'in m,m;return m['result']
 def reverse_request(self):
  while True:
   for n,m in enumerate(self.queue):
    if m.get('type')=='request':self.reverse.append(m);return self.queue.pop(n)
   m=self.read()
   if m.get('type')=='request':self.reverse.append(m);return m
   self.queue.append(m)
 def reply(self,m,value=None,error=None):
  self.send({'type':'response','id':m['id'],**({'error':{'code':'owned','message':error}}if error else{'result':value})})
 def start_open(self,sid):return self.request('session.open',{'sessionId':sid,'commandName':'convert-image-to-ascii','preferences':{},'argumentValues':{}})
 def open(self,sid,selection=None,error=None):
  req=self.start_open(sid);rpc=self.reverse_request();assert rpc['method']=='host.selection.read'and rpc['params']=={'kind':'files'};self.reply(rpc,selection or [],error);return self.response(req)['result']
 def start_action(self,sid,id,fields=None):return self.request('session.call',{'sessionId':sid,'operation':'extension:invoke-action','payload':{'actionId':id,**({'formValues':fields}if fields is not None else{})}})
 def invoke(self,sid,id,fields=None):
  m=self.response(self.start_action(sid,id,fields));assert'result'in m,m;return m['result']
 def refresh(self,sid):return self.call('session.call',{'sessionId':sid,'operation':'extension:refresh-session','payload':{}})
 def dispose(self,sid):return self.call('session.dispose',{'sessionId':sid})
 def close(self,eof=False):
  if not eof:self.call('shutdown',{})
  self.p.stdin.close();self.p.wait(timeout=5);assert self.p.returncode==0,self.p.stderr.read().decode();assert not list((self.root/'temp').iterdir());assert not list((self.root/'home').iterdir());assert not list((self.root/'data').iterdir())
def fields(v,rev,**d):return{**{'image':[],'width':'80','height':'','complex':False},**d,'__formToken':v['root']['props']['formToken'],'__formRevision':rev}
def action(v,kind):return next(a['id']for a in v['actions']if a['id'].endswith(':'+kind))
def change(v):return v['root']['children'][0]['props']['actionId']
def check(x):
 global checks
 assert x;checks+=1

def production(binary):
 p=Pipe(binary,'production-'+uuid.uuid4().hex[:8]);v=p.open('p');check(v['root']['type']=='Form');check(v['root']['children'][1]['props']['value']=='80');check(p.networkDenied)
 old=action(v,'submit');v=p.invoke('p',old,fields(v,0));check(v['message']=='Error: Please select an image file.');check(p.invoke('p',old,fields(v,0))['mode']=='unchanged');check(p.refresh('p')==v)
 check(len(p.reverse)==1);p.close(eof=True);return {'checks':checks,'externalNetworkDenied':True,'actualProductionProcess':True,'liveUpload':False}
def worker(binary):
 tls=TLS()
 try:
  p=Pipe(binary,'worker-'+uuid.uuid4().hex[:8],tls.endpoint)
  fixture=p.root/'selected/Owned.PNG';fixture.write_bytes(bytes([0,1,2,255]));image=[str(fixture)]
  for n,selected in enumerate([[],[str(fixture),str(fixture)],[str(p.root/'selected/no.gif')]]):
   v=p.open('sel'+str(n),selected);check(v['root']['children'][0]['props']['value']==[]);p.dispose('sel'+str(n))
  v=p.open('fail',error='owned selection failure');check(v['root']['type']=='Form');p.dispose('fail')
  v=p.open('s',image);check(v['root']['children'][0]['props']['value']==image)
  old=action(v,'submit');ch=change(v)
  for rev,width in enumerate(['80','81','80'],1):v=p.invoke('s',ch,fields(v,rev,image=image,width=width))
  check(p.invoke('s',old,fields(v,0,image=image))['mode']=='unchanged')
  stale=fields(v,2,image=image,width='81');check(p.invoke('s',ch,stale)['mode']=='unchanged')
  v=p.invoke('s',action(v,'submit'),fields(v,4,image=image,width='bad',height='2.5',complex=True));check(v['message']=='Error: Width must be a positive number.');check(v['root']['children'][2]['props']['value']=='2.5');check(v['root']['children'][4]['props']['value']is True)
  tls.mode('json');v=p.invoke('s',action(v,'submit'),fields(v,1,image=image,width='0x10',height='1.5',complex=True));parts=tls.next();check([x['name']for x in parts]==['file','complex','width','height']);check(parts[0]['filename']=='Owned.PNG'and parts[0]['bytes']=='000102ff');check(bytes.fromhex(parts[2]['bytes'])==b'16');check(v['root']['props']['literalText']==ASCII);check(v['root']['props']['backActionId']==action(v,'back'))
  (Q/'native-views.json').write_text(json.dumps([v]));
  invalid=[None,True,False,{}, {'ok':False},{'ok':True,'error':'contradiction'},{'ok':True,'message':None},{'ok':True,'unknown':0}]
  for ack in invalid:
   old=action(v,'copy');req=p.start_action('s',old);rpc=p.reverse_request();check(rpc['method']=='host.clipboard.write'and rpc['params']=={'text':ASCII});p.reply(rpc,ack);v=p.response(req)['result'];check(v['actions'][0]['title']=='Retry Copy Exact Ascii');check(p.invoke('s',old)['mode']=='unchanged');check(p.refresh('s')==v)
  (Q/'unknown-copy-view.json').write_text(json.dumps(v))
  req=p.start_action('s',action(v,'copy'));rpc=p.reverse_request();check('error'in p.response(p.start_action('s',action(v,'copy'))));check(p.refresh('s')['mode']=='unchanged');p.send({'type':'cancel','requestId':req,'reason':'owned cancel'});check('error'in p.response(req));p.reply(rpc,{'ok':True});v=p.refresh('s');check(v['actions'][0]['title']=='Retry Copy Exact Ascii')
  for _ in range(2):
   old=action(v,'copy');req=p.start_action('s',old);rpc=p.reverse_request();check(rpc['params']=={'text':ASCII});p.reply(rpc,{'ok':True,'message':'owned'});v=p.response(req)['result'];check(v['actions'][0]['title']=='Copy Ascii Image'and action(v,'copy')!=old);check(p.invoke('s',old)['mode']=='unchanged')
  v=p.invoke('s',action(v,'back'));check(v['root']['type']=='Form'and v['root']['children'][1]['props']['value']=='0x10')
  # Cancel a dispatched upload, deliver a late response, and retry frozen bytes.
  gate=tls.mode('hold');old=action(v,'submit');req=p.start_action('s',old,fields(v,0,image=image,width='0x10',height='1.5',complex=True));first=tls.next();check(p.refresh('s')['mode']=='unchanged');p.send({'type':'cancel','requestId':req,'reason':'owned interrupted POST'});check('error'in p.response(req));gate.set();v=p.refresh('s');check('outcome unknown'in v['message']);check(p.invoke('s',old)['mode']=='unchanged');fixture.write_bytes(b'changed during unknown outcome')
  check(p.refresh('s')==v);tls.mode('text');retry=action(v,'retry');v=p.invoke('s',retry);second=tls.next();check(first==second);check(v['root']['props']['literalText']==ASCII);check(p.invoke('s',retry)['mode']=='unchanged')
  v=p.invoke('s',action(v,'back'));check(v['root']['children'][1]['props']['value']=='0x10')
  for mode in ['error','object']:
   tls.mode(mode);old=action(v,'submit');v=p.invoke('s',old,fields(v,0,image=image,width='0x10',height='1.5',complex=True));tls.next();check(v['root']['type']=='Form');check('private server'not in v['message']);check(not any(a['id'].endswith(':retry')for a in v['actions']));check(p.invoke('s',old)['mode']=='unchanged')
  tls.mode('text');v=p.invoke('s',action(v,'submit'),fields(v,0,image=image,width='0x10',height='1.5',complex=True));tls.next();check(v['root']['type']=='Detail');p.dispose('s')
  # Canceled opens never commit SDK state; adapter captures cancellation outside it.
  for i in range(140):
   sid='cancel'+str(i);req=p.start_open(sid);rpc=p.reverse_request();p.send({'type':'cancel','requestId':req,'reason':'owned canceled open'});check('error'in p.response(req));p.reply(rpc,[])
  for i in range(16):p.open('capacity'+str(i))
  check('error'in p.response(p.start_open('overflow')))
  for i in range(16):p.dispose('capacity'+str(i))
  p.open('after-capacity');p.dispose('after-capacity');check(tls.received.empty());p.close();
  # EOF while selection is pending cancels reverse RPC and cleans the process.
  p=Pipe(binary,'eof-'+uuid.uuid4().hex[:8],tls.endpoint);p.start_open('eof');p.reverse_request();p.close(eof=True)
  return {'checks':checks,'canceledOpens':140,'capacity':16,'actualTestExecutablePipes':True,'productionOriginOverride':False,'ownedTLS':True,'noLiveServiceUpload':True}
 finally:tls.close()
if __name__=='__main__':
 binary=Path(sys.argv[-1]);report=worker(binary)if'--worker'in sys.argv else production(binary);print(json.dumps(report))
