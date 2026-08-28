#!/usr/bin/env python3
import base64, ctypes, json, ntpath, os, signal, socket, subprocess, threading, time, shlex, re, sys, traceback
from pathlib import Path
from urllib.parse import urlparse
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from urllib.request import Request, urlopen
from urllib.error import HTTPError

ROOT = Path(getattr(sys,'_MEIPASS',Path(__file__).resolve().parent))
DATA_ROOT = Path(os.environ.get('VIBEWING_DATA_DIR') or Path(__file__).resolve().parent)
DATA_ROOT.mkdir(parents=True,exist_ok=True)
DATA = DATA_ROOT / 'projects.json'
SETTINGS = DATA_ROOT / 'settings.json'
CHATS = DATA_ROOT / 'chats.json'
LOGS = DATA_ROOT / 'logs'
CREDENTIALS = DATA_ROOT / 'credentials.json'
WEB = ROOT / 'index.html'
LOCK = threading.Lock()
AI_TASKS = {}
AI_TASKS_LOCK = threading.Lock()
BUILD_TASKS = {}
BUILD_TASKS_LOCK = threading.Lock()
SERVICE_NAME = 'vibewing-nvidia-api-key'
ACCESS_TOKEN = os.environ.get('VIBEWING_ACCESS_TOKEN','')

def credential_account():
    return os.getenv('USER','vibewing')

class DATA_BLOB(ctypes.Structure):
    _fields_=[('cbData',ctypes.c_ulong),('pbData',ctypes.POINTER(ctypes.c_ubyte))]

def windows_credential_store():
    try:
        data=json.loads(CREDENTIALS.read_text(encoding='utf-8')) if CREDENTIALS.exists() else {}
        return data if isinstance(data,dict) else {}
    except Exception:
        return {}

def save_windows_credential_store(data):
    temporary=CREDENTIALS.with_suffix('.tmp')
    temporary.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
    temporary.replace(CREDENTIALS)

def windows_crypto_api():
    crypt32=ctypes.windll.crypt32; kernel32=ctypes.windll.kernel32
    blob_pointer=ctypes.POINTER(DATA_BLOB)
    crypt32.CryptProtectData.argtypes=[blob_pointer,ctypes.c_wchar_p,blob_pointer,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_ulong,blob_pointer]
    crypt32.CryptProtectData.restype=ctypes.c_int
    crypt32.CryptUnprotectData.argtypes=[blob_pointer,ctypes.c_void_p,blob_pointer,ctypes.c_void_p,ctypes.c_void_p,ctypes.c_ulong,blob_pointer]
    crypt32.CryptUnprotectData.restype=ctypes.c_int
    kernel32.LocalFree.argtypes=[ctypes.c_void_p]; kernel32.LocalFree.restype=ctypes.c_void_p
    return crypt32,kernel32

def dpapi_encrypt(value):
    raw=value.encode('utf-8'); source_buffer=ctypes.create_string_buffer(raw)
    source=DATA_BLOB(len(raw),ctypes.cast(source_buffer,ctypes.POINTER(ctypes.c_ubyte))); output=DATA_BLOB()
    crypt32,kernel32=windows_crypto_api()
    if not crypt32.CryptProtectData(ctypes.byref(source),'VibeWing API Key',None,None,None,1,ctypes.byref(output)):
        raise ctypes.WinError()
    try: return base64.b64encode(ctypes.string_at(output.pbData,output.cbData)).decode('ascii')
    finally: kernel32.LocalFree(output.pbData)

def dpapi_decrypt(value):
    raw=base64.b64decode(value); source_buffer=ctypes.create_string_buffer(raw)
    source=DATA_BLOB(len(raw),ctypes.cast(source_buffer,ctypes.POINTER(ctypes.c_ubyte))); output=DATA_BLOB()
    crypt32,kernel32=windows_crypto_api()
    if not crypt32.CryptUnprotectData(ctypes.byref(source),None,None,None,None,1,ctypes.byref(output)):
        raise ctypes.WinError()
    try: return ctypes.string_at(output.pbData,output.cbData).decode('utf-8')
    finally: kernel32.LocalFree(output.pbData)

def windows_credential(service):
    try:
        encrypted=windows_credential_store().get(service,'')
        return dpapi_decrypt(encrypted) if encrypted else ''
    except Exception:
        return ''

def set_windows_credential(service,key):
    data=windows_credential_store()
    if key: data[service]=dpapi_encrypt(key)
    else: data.pop(service,None)
    if data: save_windows_credential_store(data)
    elif CREDENTIALS.exists(): CREDENTIALS.unlink()

def generate_commit_message(task_id, cwd):
    try:
        r=run('git diff --cached --stat && git diff --cached --numstat && git diff --cached --unified=1',cwd,30)
        settings=load_settings(); selected=settings.get('default_chat_model',''); pid,_,selected_model=selected.partition('::')
        provider=next((p for p in settings.get('providers',[]) if p.get('id')==pid),None)
        if not provider: provider=next(iter(settings.get('providers',[])),{})
        if not provider: raise RuntimeError('请先在设置中添加模型平台')
        model=selected_model or provider.get('model') or settings.get('model',''); key=provider_key(provider.get('id',''))
        if not key: raise RuntimeError('默认模型平台尚未配置 API Key')
        diff=r['stdout'].strip()
        if not diff: raise RuntimeError('请先暂存需要提交的文件')
        # 避免大 diff 让模型请求异常缓慢，提交信息只需要代表性的变更摘要。
        diff=diff[:18000]
        prompt='You generate Git commit messages. Analyze the staged changes and return two versions of the same specific commit message: English and Simplified Chinese. Keep the Conventional Commit prefix before the colon exactly identical in both versions (for example feat(models):). Translate only the description after the colon. Return only valid JSON in this exact shape: {"en":"feat(scope): English description","cn":"feat(scope): 中文描述"}. Do not add Markdown or explanations.\n\nSTAGED CHANGES:\n'+diff
        payload=json.dumps({'model':model,'messages':[{'role':'system','content':'Return only JSON containing en and cn commit messages. Keep their Conventional Commit prefixes identical.'},{'role':'user','content':prompt}],'temperature':0.2,'max_tokens':768}).encode()
        endpoint=(provider.get('base_url') or 'https://integrate.api.nvidia.com/v1').rstrip('/')+'/chat/completions'
        req=Request(endpoint,data=payload,headers={'Authorization':'Bearer '+key,'Content-Type':'application/json'})
        out=json.loads(urlopen(req,timeout=120).read()); choice=(out.get('choices') or [{}])[0]; message=choice.get('message') or {}
        # Reasoning models may return an explanation separately from the final answer.
        # Never treat arbitrary reasoning prose as a commit message.
        candidates=[message.get('content'),choice.get('text'),out.get('output_text'),message.get('reasoning_content')]
        pattern=re.compile(r'^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9._/-]+\))?!?:\s+\S.+$',re.I)
        messages={}
        for content in candidates:
            if not isinstance(content,str): continue
            match=re.search(r'\{\s*"en"\s*:\s*"((?:[^"\\]|\\.)*)"\s*,\s*"cn"\s*:\s*"((?:[^"\\]|\\.)*)"\s*\}',content,re.S)
            if match:
                try: messages={'en':json.loads('"'+match.group(1)+'"'),'cn':json.loads('"'+match.group(2)+'"')}
                except Exception: messages={}
            if messages: break
        en=messages.get('en','').strip(); cn=messages.get('cn','').strip()
        if not pattern.match(en) or not pattern.match(cn): raise RuntimeError('模型没有返回有效的中英文提交信息，请重试或切换模型')
        if en.split(':',1)[0].lower()!=cn.split(':',1)[0].lower(): raise RuntimeError('模型返回的中英文提交类型不一致，请重试')
        with AI_TASKS_LOCK: AI_TASKS[task_id]={'status':'done','message':en,'messages':{'en':en,'cn':cn}}
    except Exception as e:
        with AI_TASKS_LOCK: AI_TASKS[task_id]={'status':'error','error':str(e)}

def load_settings():
    defaults={'model':'','models':[],'providers':[],'theme':{'accent':'#20bdb7','bg':'#f2fbfb','card':'#ffffff','preset':'winglight'},'language':'zh-CN','default_chat_model':''}
    try:
        saved=json.loads(SETTINGS.read_text()) if SETTINGS.exists() else {}
        defaults.update(saved)
    except Exception: pass
    theme=defaults.get('theme') or {}
    if theme.get('preset')=='wingdark' and theme.get('bg')=='#07191d' and theme.get('card')=='#0e282d':
        defaults['theme']={**theme,'accent':'#42d5cf','bg':'#0b1220','card':'#142331'}
    if defaults.get('model') and defaults['model'] not in defaults['models']: defaults['models'].append(defaults['model'])
    for p in defaults.get('providers',[]):
        p.setdefault('models',[p.get('model','')]); p['models']=[m for m in p['models'] if m]
        if p.get('model') and p['model'] not in p['models']: p['models'].append(p['model'])
    return defaults

def save_settings(data): SETTINGS.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')

def load_chats():
    """Load saved VibeWing conversations."""
    try:
        chats=json.loads(CHATS.read_text()) if CHATS.exists() else []
    except Exception:
        return []
    return chats if isinstance(chats,list) else []

def get_api_key():
    if os.getenv('NVIDIA_API_KEY'): return os.getenv('NVIDIA_API_KEY')
    if os.name=='nt': return windows_credential(SERVICE_NAME)
    if os.name == 'posix' and subprocess.run(['sh','-c','command -v security >/dev/null 2>&1']).returncode == 0:
        r=subprocess.run(['security','find-generic-password','-s',SERVICE_NAME,'-w'],capture_output=True,text=True)
        if r.returncode==0 and r.stdout.strip(): return r.stdout.strip()
    return ''

def set_api_key(key):
    if os.name=='nt':
        set_windows_credential(SERVICE_NAME,key); return
    if os.name == 'posix' and subprocess.run(['sh','-c','command -v security >/dev/null 2>&1']).returncode == 0:
        subprocess.run(['security','delete-generic-password','-s',SERVICE_NAME],capture_output=True)
        if key: subprocess.run(['security','add-generic-password','-a',credential_account(),'-s',SERVICE_NAME,'-w',key],capture_output=True)
        return
    os.environ['NVIDIA_API_KEY']=key

def provider_key(pid):
    if pid=='nvidia': return get_api_key()
    if os.name=='nt': return windows_credential('vibewing-provider-'+pid)
    if os.name=='posix' and subprocess.run(['sh','-c','command -v security >/dev/null 2>&1']).returncode==0:
        service='vibewing-provider-'+pid
        r=subprocess.run(['security','find-generic-password','-s',service,'-w'],capture_output=True,text=True)
        if r.returncode==0 and r.stdout.strip(): return r.stdout.strip()
    return os.getenv('VIBEWING_KEY_'+pid.upper(),'')

def set_provider_key(pid,key):
    if pid=='nvidia': return set_api_key(key)
    if os.name=='nt':
        set_windows_credential('vibewing-provider-'+pid,key); return
    if os.name=='posix' and subprocess.run(['sh','-c','command -v security >/dev/null 2>&1']).returncode==0:
        service='vibewing-provider-'+pid; subprocess.run(['security','delete-generic-password','-s',service],capture_output=True)
        if key: subprocess.run(['security','add-generic-password','-a',credential_account(),'-s',service,'-w',key],capture_output=True)
    else: os.environ['VIBEWING_KEY_'+pid.upper()]=key

def provider_config(pid,model=''):
    s=load_settings(); p=next((x for x in s.get('providers',[]) if x.get('id')==pid),None)
    if not p: p=next(iter(s.get('providers',[])),{})
    models=p.get('models') or []
    selected=model or p.get('model') or (models[0] if models else s.get('model',''))
    return p, provider_key(p.get('id','')), selected

def bounded_chat_messages(raw, max_messages=12, max_total_chars=24000):
    """Return an API-safe system + alternating user/assistant conversation.

    Saved chats can contain old action cards, duplicate error replies, or malformed
    roles. OpenAI-compatible providers differ in how tolerant they are, so make
    this backend boundary deterministic before every request.
    """
    system=None; conversation=[]
    for item in raw if isinstance(raw,list) else []:
        if not isinstance(item,dict): continue
        role=item.get('role')
        if role not in ('system','user','assistant'): continue
        content=item.get('content','')
        if not isinstance(content,str): content=str(content) if content is not None else ''
        content=content.strip()
        if not content: continue
        if role=='system':
            # A system instruction is metadata, never part of role alternation.
            if system is None: system={'role':'system','content':content[:16000]}
            continue
        if role=='assistant' and (
            content.startswith(('请求失败：','Request failed:','模型没有返回','模型服务在 10 分钟内没有响应'))
            or '模型平台网关超时或暂时不可用' in content
        ):
            # Local transport errors are UI state, not assistant knowledge. Feeding
            # them back wastes context and can distort role history after retries.
            continue
        # A valid conversation always starts with a user message.
        if not conversation and role!='user': continue
        if conversation and conversation[-1]['role']==role:
            # Preserve both messages while restoring strict alternation. Keep the
            # newest part if old saved content exceeds the per-message limit.
            conversation[-1]['content']=(conversation[-1]['content']+'\n\n'+content)[-16000:]
        else:
            conversation.append({'role':role,'content':content[:16000]})

    # A completion request must end with the user message being answered.
    if conversation and conversation[-1]['role']=='assistant': conversation.pop()
    if not conversation: return [system] if system else []

    # Keep the newest context. Because the sequence ends in user, removing a
    # leading assistant after slicing preserves user/assistant alternation.
    conversation=conversation[-max_messages:]
    if conversation and conversation[0]['role']=='assistant': conversation.pop(0)

    used=len(system['content']) if system else 0; recent=[]
    for message in reversed(conversation):
        remaining=max_total_chars-used
        if remaining<=0: break
        content=message['content']
        if len(content)>remaining:
            if recent: break
            content=content[-remaining:]
        recent.append({'role':message['role'],'content':content}); used+=len(content)
    conversation=list(reversed(recent))
    if conversation and conversation[0]['role']=='assistant': conversation.pop(0)
    if conversation and conversation[-1]['role']=='assistant': conversation.pop()
    return ([system] if system else [])+conversation

def messages_for_model(messages, model):
    """Adapt system instructions for models that only accept dialogue roles."""
    if not re.search(r'(^|[\/_-])(guard|safety|moderation)([\/_-]|$)',model or '',re.I):
        return messages
    if messages and messages[0].get('role')=='system':
        system=messages[0].get('content',''); messages=[dict(x) for x in messages[1:]]
        if messages and messages[0].get('role')=='user':
            messages[0]['content']=system+'\n\n'+messages[0].get('content','')
    return messages

def stream_chat_completion(req, timeout=600):
    """Read OpenAI-compatible SSE, including NVIDIA reasoning-model fields."""
    content=[]; reasoning=[]; calls={}; legacy={'name':'','arguments':''}; finish_reason=''
    with urlopen(req,timeout=timeout) as response:
        content_type=(response.headers.get('Content-Type') or '').lower()
        if 'text/event-stream' not in content_type:
            out=json.loads(response.read())
            choice=(out.get('choices') or [{}])[0]
            return choice.get('message') or {'content':choice.get('text') or ''},choice.get('finish_reason') or ''
        for raw_line in response:
            line=raw_line.decode('utf-8','replace').strip()
            if not line.startswith('data:'): continue
            data=line[5:].strip()
            if not data or data=='[DONE]': continue
            try: chunk=json.loads(data)
            except json.JSONDecodeError: continue
            choice=(chunk.get('choices') or [{}])[0]; finish_reason=choice.get('finish_reason') or finish_reason
            delta=choice.get('delta') or {}
            if isinstance(delta.get('content'),str): content.append(delta['content'])
            # NVIDIA reasoning models stream their analysis separately. Keep it
            # only as a diagnostic fallback; prefer the final answer in content.
            for key in ('reasoning_content','reasoning'):
                if isinstance(delta.get(key),str): reasoning.append(delta[key])
            for tool in delta.get('tool_calls') or []:
                index=tool.get('index',0); current=calls.setdefault(index,{'id':'','type':'function','function':{'name':'','arguments':''}})
                if tool.get('id'): current['id']+=tool['id']
                function=tool.get('function') or {}
                current['function']['name']+=function.get('name') or ''
                current['function']['arguments']+=function.get('arguments') or ''
            old=delta.get('function_call') or {}
            legacy['name']+=old.get('name') or ''; legacy['arguments']+=old.get('arguments') or ''
    message={'content':''.join(content),'reasoning_content':''.join(reasoning),'tool_calls':[calls[k] for k in sorted(calls)]}
    if legacy['name']: message['function_call']=legacy
    return message,finish_reason

def nonstream_chat_completion(endpoint, headers, payload, timeout=600):
    """Read the final assistant answer from an OpenAI-compatible response."""
    fallback=dict(payload); fallback['stream']=False
    req=Request(endpoint,data=json.dumps(fallback).encode(),headers={**headers,'Accept':'application/json'})
    out=json.loads(urlopen(req,timeout=timeout).read())
    choice=(out.get('choices') or [{}])[0]; message=choice.get('message') or {}
    return str(message.get('content') or choice.get('text') or out.get('output_text') or '').strip()

def load():
    if not DATA.exists(): return []
    try: return json.loads(DATA.read_text(encoding='utf-8'))
    except Exception: return []

def save(items):
    temporary=DATA.with_suffix('.tmp')
    temporary.write_text(json.dumps(items, ensure_ascii=False, indent=2),encoding='utf-8')
    temporary.replace(DATA)

def normalize_project_path(value):
    value=str(value or '').strip().strip('"')
    if os.name!='nt': return os.path.expanduser(value)
    value=value.replace('/','\\')
    # Some browser/file-picker representations use /D:/folder. Accept that
    # form, but reject drive-relative paths such as /folder: their drive is
    # ambiguous and often caused VibeWing to start in its own install drive.
    if re.match(r'^\\[A-Za-z]:\\',value): value=value[1:]
    if value.startswith('\\') and not value.startswith('\\\\'):
        raise ValueError('Windows 路径缺少盘符，请填写完整路径，例如 D:\\Projects\\MyApp')
    if re.match(r'^[A-Za-z]:[^\\]',value):
        raise ValueError('Windows 路径必须包含盘符和反斜杠，例如 D:\\Projects\\MyApp')
    return ntpath.normpath(value) if value else ''

def run(cmd, cwd, timeout=12):
    try:
        p = subprocess.run(cmd, cwd=cwd, shell=True, capture_output=True, text=True, timeout=timeout)
        return {'code': p.returncode, 'stdout': p.stdout[-12000:], 'stderr': p.stderr[-12000:]}
    except Exception as e: return {'code': -1, 'stdout': '', 'stderr': str(e)}

def run_args(args, cwd, timeout=30):
    try:
        p=subprocess.run(args,cwd=cwd,capture_output=True,text=True,timeout=timeout)
        return {'code':p.returncode,'stdout':p.stdout[-20000:],'stderr':p.stderr[-20000:]}
    except subprocess.TimeoutExpired as e:
        stdout=e.stdout.decode('utf-8','replace') if isinstance(e.stdout,bytes) else (e.stdout or '')
        stderr=e.stderr.decode('utf-8','replace') if isinstance(e.stderr,bytes) else (e.stderr or '')
        return {'code':124,'stdout':stdout[-20000:],'stderr':stderr[-20000:],'error':f'命令执行超过 {timeout} 秒，已停止等待'}
    except Exception as e: return {'code':-1,'stdout':'','stderr':str(e)}

def git_files(cwd):
    try:
        p=subprocess.run(['git','status','--porcelain=v1','-z'],cwd=cwd,capture_output=True,timeout=10)
        raw=p.stdout.decode('utf-8','replace').split('\0'); files=[]; i=0
        while i < len(raw) and raw[i]:
            entry=raw[i]; x,y=entry[0],entry[1]; name=entry[3:]; i+=1
            if x in ('R','C') and i < len(raw) and raw[i]:
                original=raw[i]; i+=1
            else: original=''
            files.append({'path':name,'original_path':original,'x':x,'y':y,'staged':x not in (' ','?'),'unstaged':y!=' ' or x=='?'})
        return files
    except Exception: return []

def git_branches(cwd):
    r=run_args(['git','for-each-ref','--format=%(refname:short)','refs/heads'],cwd)
    return [x.strip() for x in r['stdout'].splitlines() if x.strip()]

def git_commit_url(cwd, commit):
    remote=run_args(['git','remote','get-url','origin'],cwd,15)
    if remote['code']!=0: return ''
    url=remote['stdout'].strip()
    if url.startswith('git@') and ':' in url:
        host,path=url[4:].split(':',1); url=f'https://{host}/{path}'
    elif url.startswith('ssh://git@'):
        url='https://'+url[len('ssh://git@'):]
    if url.endswith('.git'): url=url[:-4]
    if not url.startswith(('http://','https://')): return ''
    return url.rstrip('/')+'/commit/'+commit

def project_git_cwd(project, scope):
    """Resolve a configured frontend/backend directory to its containing Git root."""
    if scope not in ('frontend','backend'): raise RuntimeError('无效 Git 范围')
    configured=os.path.expanduser(project.get(scope+'_path') or project.get('path') or '')
    if not configured or not Path(configured).is_dir(): raise RuntimeError(('前端' if scope=='frontend' else '后端')+'目录不存在：'+configured)
    result=run_args(['git','rev-parse','--show-toplevel'],configured,10)
    if result['code']!=0: raise RuntimeError(('前端' if scope=='frontend' else '后端')+'目录不在 Git 仓库中：'+configured)
    return result['stdout'].strip()

def port_open(port):
    if not port: return False
    s=socket.socket(); s.settimeout(.25)
    try: s.connect(('127.0.0.1', int(port))); return True
    except Exception: return False
    finally: s.close()

def pid_alive(pid):
    try:
        os.kill(int(pid), 0)
        if os.name == 'posix':
            r=subprocess.run(['ps','-o','stat=','-p',str(pid)],capture_output=True,text=True,timeout=2)
            if not r.stdout.strip() or r.stdout.strip().startswith('Z'): return False
        return True
    except Exception: return False

def pid_for_port(port):
    if not port: return None
    try:
        if os.name == 'posix':
            r=subprocess.run(['lsof','-nP','-tiTCP:'+str(port),'-sTCP:LISTEN'],capture_output=True,text=True,timeout=3)
            return int(r.stdout.splitlines()[0]) if r.stdout.strip() else None
        r=subprocess.run(['netstat','-ano','-p','tcp'],capture_output=True,text=True,timeout=3)
        for line in r.stdout.splitlines():
            parts=line.split()
            if len(parts)>=5 and parts[1].rsplit(':',1)[-1]==str(port) and parts[3].upper()=='LISTENING': return int(parts[4])
    except Exception: pass
    return None

def start_service(project, key):
    cmd = project.get(key + '_cmd', '').strip()
    if not cmd: return {'ok': False, 'error': '未配置启动命令'}
    cwd=os.path.expanduser(project.get(key+'_path') or project.get('path',''))
    if not os.path.isdir(cwd): return {'ok':False,'error':f'{key} 工作目录不存在：{cwd}'}
    if cmd.strip().startswith(('npm ','npm\t','pnpm ','yarn ')) and not os.path.isfile(os.path.join(cwd,'package.json')):
        return {'ok':False,'error':f'启动目录中没有 package.json：{cwd}\n请编辑项目，为前端选择正确的工作目录。'}
    pid_key = key + '_pid'
    if pid_alive(project.get(pid_key)) or port_open(project.get(key+'_port')): return {'ok': True, 'message': '已经在运行'}
    log = LOGS; log.mkdir(exist_ok=True)
    log_path=log / f"{project['id']}-{key}.log"
    f = open(log_path, 'a', buffering=1, encoding='utf-8', errors='replace')
    f.write(f"\n[{time.strftime('%Y-%m-%d %H:%M:%S')}] $ {cmd}\n")
    if os.name == 'nt': argv=['cmd.exe','/d','/s','/c',cmd]
    else:
        shell=os.getenv('SHELL') or '/bin/zsh'
        argv=[shell,'-l','-c',cmd]
    try:
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name=='nt' else 0
        p = subprocess.Popen(argv, cwd=cwd, stdout=f, stderr=subprocess.STDOUT, start_new_session=os.name!='nt', creationflags=creationflags, env=os.environ.copy())
    except Exception as e:
        f.write('启动失败：'+str(e)+'\n'); f.close(); return {'ok':False,'error':str(e)}
    f.close()
    project[pid_key] = p.pid
    project[key + '_started_at'] = int(time.time())
    time.sleep(.35)
    code=p.poll()
    if code is not None and not port_open(project.get(key+'_port')):
        project[pid_key]=None
        tail=service_log(project,key,5000)
        return {'ok':False,'error':f'启动命令已退出（代码 {code}）\n\n{tail[-3000:]}'}
    return {'ok': True, 'pid': p.pid}

def stop_service(project, key):
    pid_key=key+'_pid'; pid=project.get(pid_key) or pid_for_port(project.get(key+'_port'))
    if not pid or not pid_alive(pid): project[pid_key]=None; return {'ok': True, 'message': '未运行'}
    try:
        if os.name == 'nt': subprocess.run(['taskkill','/PID',str(pid),'/T'],capture_output=True,timeout=5)
        else: os.killpg(os.getpgid(int(pid)), signal.SIGTERM)
        for _ in range(20):
            if not pid_alive(pid): break
            time.sleep(.1)
        if pid_alive(pid):
            try:
                if os.name == 'nt': subprocess.run(['taskkill','/PID',str(pid),'/T','/F'],capture_output=True,timeout=5)
                else: os.killpg(os.getpgid(int(pid)), signal.SIGKILL)
            except ProcessLookupError: pass
        project[pid_key]=None; return {'ok': True}
    except Exception as e: return {'ok': False, 'error': str(e)}

def project_view(p):
    with BUILD_TASKS_LOCK: building=any(BUILD_TASKS.get(f"{p['id']}:{mode}",{}).get('status') in ('starting','running') for mode in ('production','test'))
    return {**p, 'frontend_running': pid_alive(p.get('frontend_pid')) or port_open(p.get('frontend_port')), 'backend_running': pid_alive(p.get('backend_pid')) or port_open(p.get('backend_port')), 'frontend_port_open': port_open(p.get('frontend_port')), 'backend_port_open': port_open(p.get('backend_port')), 'frontend_building':building}

def service_log(project,key,limit=30000):
    path=LOGS/f"{project['id']}-{key}.log"
    if not path.exists(): return ''
    try:
        with path.open('rb') as f:
            f.seek(0,2); size=f.tell(); f.seek(max(0,size-limit)); return f.read().decode('utf-8','replace')
    except Exception as e: return '读取日志失败：'+str(e)

def project_diagnostic_context(project):
    """Collect a bounded, read-only project snapshot without exposing secret values."""
    root=Path(os.path.expanduser(project.get('path','')))
    frontend=Path(os.path.expanduser(project.get('frontend_path') or project.get('path','')))
    lines=[f"Project: {project.get('name','')}",f"Root: {root}",f"Frontend directory: {frontend}",f"Backend directory: {project.get('backend_path') or root}"]
    try:
        names=sorted(x.name+('/' if x.is_dir() else '') for x in root.iterdir() if not x.name.startswith('.'))[:80]
        lines.append('Root entries: '+', '.join(names))
    except Exception as e: lines.append('Root entries unavailable: '+str(e))
    package=frontend/'package.json'
    if package.exists():
        try:
            data=json.loads(package.read_text()); scripts=data.get('scripts',{}) if isinstance(data.get('scripts'),dict) else {}
            lines.append('package.json scripts:\n'+json.dumps(scripts,ensure_ascii=False,indent=2))
            deps=list((data.get('dependencies') or {}).keys()); dev_deps=list((data.get('devDependencies') or {}).keys())
            lines.append('Dependencies: '+', '.join(deps[:100])); lines.append('Dev dependencies: '+', '.join(dev_deps[:100]))
        except Exception as e: lines.append('package.json read error: '+str(e))
    else: lines.append('package.json: not found in frontend directory')
    lockfiles=[name for name in ('pnpm-lock.yaml','package-lock.json','yarn.lock','bun.lock','bun.lockb') if (frontend/name).exists()]
    lines.append('Detected lockfiles: '+(', '.join(lockfiles) if lockfiles else 'none'))
    configs=[]
    for pattern in ('vite.config.*','vue.config.*','next.config.*','nuxt.config.*','tsconfig*.json','.env.example','.env.test','.env.staging','.env.production'):
        configs.extend(x.name for x in frontend.glob(pattern))
    lines.append('Detected configuration files: '+(', '.join(sorted(set(configs))) if configs else 'none'))
    if (root/'.git').exists():
        branch=run_args(['git','branch','--show-current'],str(root),5)['stdout'].strip()
        status=run_args(['git','status','--short'],str(root),5)['stdout'][:6000].strip()
        lines.append('Git branch: '+(branch or 'unknown')); lines.append('Git status:\n'+(status or 'clean'))
    return '\n'.join(lines)[:16000]

def frontend_build_command(project, mode):
    cwd=Path(os.path.expanduser(project.get('frontend_path') or project.get('path','')))
    package_path=cwd/'package.json'
    if not package_path.exists(): raise RuntimeError(f'前端目录中没有 package.json：{cwd}')
    try: scripts=json.loads(package_path.read_text()).get('scripts',{})
    except Exception as e: raise RuntimeError('无法读取 package.json：'+str(e))
    candidates=['build:prod','build:production','build'] if mode=='production' else ['build:test','build:staging','test:build']
    script=next((name for name in candidates if name in scripts),None)
    if not script: raise RuntimeError(('生产' if mode=='production' else '测试')+'打包脚本不存在')
    runner='pnpm run' if (cwd/'pnpm-lock.yaml').exists() else ('yarn' if (cwd/'yarn.lock').exists() else ('bun run' if (cwd/'bun.lockb').exists() or (cwd/'bun.lock').exists() else 'npm run'))
    return f'{runner} {script}',str(cwd)

def run_frontend_build(project, mode):
    task=f"{project['id']}:{mode}"
    try:
        cmd,cwd=frontend_build_command(project,mode); log_dir=LOGS; log_dir.mkdir(exist_ok=True); log_path=log_dir/f"{project['id']}-frontend.log"
        with log_path.open('a',buffering=1,encoding='utf-8',errors='replace') as f:
            label='生产环境' if mode=='production' else '测试环境'; f.write(f'\n[{time.strftime("%Y-%m-%d %H:%M:%S")}] 开始构建（{label}）\n$ {cmd}\n')
            argv=['cmd.exe','/d','/s','/c',cmd] if os.name=='nt' else [os.getenv('SHELL') or '/bin/zsh','-l','-c',cmd]
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name=='nt' else 0
            p=subprocess.Popen(argv,cwd=cwd,stdout=f,stderr=subprocess.STDOUT,start_new_session=os.name!='nt',creationflags=creationflags,env=os.environ.copy())
            with BUILD_TASKS_LOCK: BUILD_TASKS[task]={'status':'running','command':cmd}
            code=p.wait(); f.write(f'\n[{time.strftime("%Y-%m-%d %H:%M:%S")}] '+('构建完成' if code==0 else f'构建失败（退出代码 {code}）')+'\n')
        with BUILD_TASKS_LOCK: BUILD_TASKS[task]={'status':'done' if code==0 else 'error','code':code,'command':cmd}
    except Exception as e:
        with BUILD_TASKS_LOCK: BUILD_TASKS[task]={'status':'error','error':str(e)}
        try:
            LOGS.mkdir(exist_ok=True)
            with (LOGS/f"{project['id']}-frontend.log").open('a',encoding='utf-8',errors='replace') as f: f.write('\n构建失败：'+str(e)+'\n')
        except Exception: pass

class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def handle_one_request(self):
        try: return super().handle_one_request()
        except (BrokenPipeError,ConnectionResetError): return
        except Exception as e:
            try:
                LOGS.mkdir(exist_ok=True)
                with (LOGS/'vibewing-api-errors.log').open('a',encoding='utf-8',errors='replace') as f:
                    f.write(f'\n[{time.strftime("%Y-%m-%d %H:%M:%S")}] {self.command if hasattr(self,"command") else "?"} {getattr(self,"path","")}\n{traceback.format_exc()}')
                self.send_json({'error':'VibeWing 本地服务处理失败：'+str(e)},500)
            except Exception: pass
    def authorized(self):
        return not ACCESS_TOKEN or self.headers.get('X-VibeWing-Token','')==ACCESS_TOKEN
    def reject_unauthorized(self):
        if self.authorized(): return False
        self.send_json({'error':'Unauthorized'},401); return True
    def send_json(self, data, code=200):
        raw=json.dumps(data, ensure_ascii=False).encode(); self.send_response(code); self.send_header('Content-Type','application/json; charset=utf-8'); self.send_header('Content-Length',str(len(raw))); self.end_headers(); self.wfile.write(raw)
    def send_ndjson_headers(self):
        self.send_response(200); self.send_header('Content-Type','application/x-ndjson; charset=utf-8'); self.send_header('Cache-Control','no-cache'); self.send_header('X-Accel-Buffering','no'); self.end_headers()
    def send_ndjson(self, data):
        self.wfile.write((json.dumps(data,ensure_ascii=False)+'\n').encode()); self.wfile.flush()
    def body(self): return json.loads(self.rfile.read(int(self.headers.get('Content-Length',0))) or '{}')
    def do_GET(self):
        path=urlparse(self.path).path
        if path.startswith('/api/') and self.reject_unauthorized(): return
        if path=='/api/projects':
            with LOCK: self.send_json([project_view(p) for p in load()])
        elif path=='/api/git':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); p=next((x for x in load() if x['id']==d.get('id',[''])[0]),None)
            if not p: return self.send_json({'error':'项目不存在'},404)
            r=run('git status --short && printf "\\n---\\n" && git branch --show-current',p['path']); self.send_json(r)
        elif path=='/api/git-panel':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); p=next((x for x in load() if x['id']==d.get('id',[''])[0]),None)
            if not p: return self.send_json({'error':'项目不存在'},404)
            scope=d.get('scope',['frontend'])[0]
            try: cwd=project_git_cwd(p,scope)
            except RuntimeError as e: return self.send_json({'error':str(e)},400)
            branch=run_args(['git','branch','--show-current'],cwd)['stdout'].strip()
            self.send_json({'files':git_files(cwd),'branch':branch,'branches':git_branches(cwd),'scope':scope,'repo_root':cwd})
        elif path=='/api/git-workspaces':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); p=next((x for x in load() if x['id']==d.get('id',[''])[0]),None)
            if not p: return self.send_json({'error':'项目不存在'},404)
            roots={}
            for scope in ('frontend','backend'):
                try: roots[scope]=project_git_cwd(p,scope)
                except RuntimeError: roots[scope]=''
            available=[scope for scope in ('frontend','backend') if roots[scope]]
            distinct=len(set(roots[scope] for scope in available))>1
            default_scope='frontend' if roots['frontend'] else ('backend' if roots['backend'] else 'frontend')
            self.send_json({'roots':roots,'available':available,'distinct':distinct,'default_scope':default_scope})
        elif path=='/api/settings':
            s=load_settings(); public=[]
            for p in s.get('providers',[]): public.append({**p,'configured':bool(provider_key(p.get('id','')))})
            self.send_json({'nvidia_configured': bool(get_api_key()), **s,'providers':public})
        elif path=='/api/chats':
            self.send_json(load_chats())
        elif path=='/api/ai-message-status':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); task_id=d.get('task_id',[''])[0]
            with AI_TASKS_LOCK: task=dict(AI_TASKS.get(task_id,{'status':'missing'}))
            self.send_json(task)
        elif path=='/api/logs':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); p=next((x for x in load() if x['id']==d.get('id',[''])[0]),None); key=d.get('service',[''])[0]
            if not p or key not in ('frontend','backend'): return self.send_json({'error':'项目或服务不存在'},404)
            self.send_json({'log':service_log(p,key)})
        elif path=='/api/project-context':
            q=urlparse(self.path).query; import urllib.parse; d=urllib.parse.parse_qs(q); p=next((x for x in load() if x['id']==d.get('id',[''])[0]),None)
            if not p: return self.send_json({'error':'项目不存在'},404)
            self.send_json({'context':project_diagnostic_context(p)})
        elif path.startswith('/assets/'):
            asset=(ROOT/path.lstrip('/')).resolve(); assets_root=(ROOT/'assets').resolve()
            if assets_root not in asset.parents or not asset.is_file(): return self.send_error(404)
            mime='image/png' if asset.suffix.lower()=='.png' else 'application/octet-stream'; raw=asset.read_bytes()
            self.send_response(200); self.send_header('Content-Type',mime); self.send_header('Content-Length',str(len(raw))); self.send_header('Cache-Control','public, max-age=86400'); self.end_headers(); self.wfile.write(raw)
        else:
            try: raw=WEB.read_bytes(); self.send_response(200); self.send_header('Content-Type','text/html; charset=utf-8'); self.send_header('Content-Length',str(len(raw))); self.end_headers(); self.wfile.write(raw)
            except: self.send_error(404)
    def do_POST(self):
        path=urlparse(self.path).path; d=self.body()
        if path.startswith('/api/') and self.reject_unauthorized(): return
        # Model requests can take up to a minute. Never hold the project-data lock
        # while waiting for the remote provider, otherwise reopening the chat
        # blocks on /api/projects and /api/settings until the answer completes.
        if path=='/api/chat':
            messages=d.get('messages',[])
            if not isinstance(messages,list) or not messages: return self.send_json({'error':'对话内容不能为空'},400)
            provider_id=d.get('provider_id','nvidia'); selected=d.get('model','').strip(); provider,key,selected=provider_config(provider_id,selected)
            messages=messages_for_model(bounded_chat_messages(messages),selected)
            conversation=[x for x in messages if x.get('role')!='system']
            if not conversation or conversation[-1].get('role')!='user': return self.send_json({'error':'对话必须以用户消息结尾'},400)
            if any(x.get('role')!=('user' if i%2==0 else 'assistant') for i,x in enumerate(conversation)):
                return self.send_json({'error':'对话角色顺序无效'},400)
            if not key: return self.send_json({'error':'请先在设置中配置该供应商的 API Key'},400)
            base=provider.get('base_url','https://integrate.api.nvidia.com/v1').rstrip('/')
            endpoint=base+'/chat/completions'; headers={'Authorization':'Bearer '+key,'Content-Type':'application/json','Accept':'text/event-stream'}
            payload_data={'model':selected,'messages':messages,'temperature':0.2,'max_tokens':2048,'stream':True}
            reasoning=d.get('reasoning_effort','auto')
            if reasoning in ('low','medium','high'): payload_data['reasoning_effort']=reasoning
            payload=json.dumps(payload_data).encode(); req=Request(endpoint,data=payload,headers=headers)
            try:
                started_at=time.monotonic()
                # AI chat keeps working after its window is closed. Allow slow
                # models ample time while still preventing a permanently hung request.
                try: response=urlopen(req,timeout=600)
                except HTTPError as first_error:
                    # reasoning_effort is common but not universal. If a provider
                    # rejects it, retry once without the optional extension.
                    if reasoning!='auto' and first_error.code==400:
                        payload_data.pop('reasoning_effort',None)
                        req=Request(endpoint,data=json.dumps(payload_data).encode(),headers=headers)
                        response=urlopen(req,timeout=600)
                    else: raise
                self.send_ndjson_headers(); content=[]; reasoning_content=[]; finish_reason=''
                with response:
                    content_type=(response.headers.get('Content-Type') or '').lower()
                    if 'text/event-stream' not in content_type:
                        out=json.loads(response.read()); choice=(out.get('choices') or [{}])[0]; msg=choice.get('message') or {}; text=str(msg.get('content') or choice.get('text') or '')
                        if text: content.append(text); self.send_ndjson({'type':'delta','content':text})
                        finish_reason=choice.get('finish_reason') or ''
                    else:
                        for raw_line in response:
                            line=raw_line.decode('utf-8','replace').strip()
                            if not line.startswith('data:'): continue
                            data=line[5:].strip()
                            if not data or data=='[DONE]': continue
                            try: chunk=json.loads(data)
                            except json.JSONDecodeError: continue
                            choice=(chunk.get('choices') or [{}])[0]; finish_reason=choice.get('finish_reason') or finish_reason; delta=choice.get('delta') or {}
                            piece=delta.get('content')
                            if isinstance(piece,str) and piece: content.append(piece); self.send_ndjson({'type':'delta','content':piece})
                            for field in ('reasoning_content','reasoning'):
                                value=delta.get(field)
                                if isinstance(value,str): reasoning_content.append(value)
                answer=''.join(content).strip(); elapsed_ms=round((time.monotonic()-started_at)*1000)
                if not answer and reasoning_content:
                    # Do not expose hidden chain-of-thought as the answer. Report a
                    # provider compatibility error instead of pretending it replied.
                    self.send_ndjson({'type':'error','error':'模型只返回了内部推理，没有最终答案；请提高输出上限或更换模型'})
                elif not answer: self.send_ndjson({'type':'error','error':'模型没有返回可展示的回答'})
                self.send_ndjson({'type':'done','elapsed_ms':elapsed_ms,'prompt_chars':sum(len(x.get('content','')) for x in messages),'message_count':len(messages),'finish_reason':finish_reason}); return
            except HTTPError as e:
                try: detail=e.read().decode('utf-8','replace')
                except Exception: detail=str(e)
                if e.code==410:
                    try:
                        body=json.loads(detail); reason=str(body.get('detail') or body.get('message') or '')
                    except Exception: reason=detail
                    return self.send_json({'error':'该模型已被平台下线，请切换其他模型','model_unavailable':True,'model':selected,'provider_id':provider_id,'reason':reason},410)
                if e.code in (502,503,504): return self.send_json({'error':'模型平台网关超时或暂时不可用，请稍后重试或切换模型','gateway_timeout':True},502)
                return self.send_json({'error':detail or str(e)},500)
            except Exception as e:
                detail=str(e)
                if isinstance(e,(TimeoutError,socket.timeout)) or 'timed out' in detail.lower(): return self.send_json({'error':'模型服务在 10 分钟内没有响应，请重试或切换模型','timeout':True},504)
                return self.send_json({'error':detail},500)
        with LOCK:
            items=load()
            if path=='/api/projects':
                try:
                    base=normalize_project_path(d.get('path','')); frontend_path=normalize_project_path(d.get('frontend_path','')) or base; backend_path=normalize_project_path(d.get('backend_path','')) or base
                except ValueError as e: return self.send_json({'error':str(e)},400)
                p={'id':str(int(time.time()*1000)),'name':d.get('name','未命名项目'),'path':base,'frontend_path':frontend_path,'backend_path':backend_path,'frontend_cmd':d.get('frontend_cmd',''),'backend_cmd':d.get('backend_cmd',''),'frontend_port':d.get('frontend_port',''),'backend_port':d.get('backend_port',''),'frontend_pid':None,'backend_pid':None}; items.append(p); save(items); return self.send_json(project_view(p))
            if path=='/api/projects/update':
                p=next((x for x in items if x['id']==d.get('id')),None)
                if not p: return self.send_json({'error':'项目不存在'},404)
                for key in ('name','frontend_cmd','backend_cmd','frontend_port','backend_port'):
                    if key in d: p[key]=d.get(key,'').strip()
                try:
                    if 'path' in d: p['path']=normalize_project_path(d.get('path',''))
                    for key in ('frontend_path','backend_path'):
                        if key in d: p[key]=normalize_project_path(d.get(key,'')) or p['path']
                except ValueError as e: return self.send_json({'error':str(e)},400)
                save(items); return self.send_json(project_view(p))
            if path=='/api/settings':
                key=d.get('nvidia_api_key','').strip()
                if key: set_api_key(key)
                if d.get('clear_key'): set_api_key('')
                s=load_settings()
                incoming=d.get('providers')
                if isinstance(incoming,list):
                    previous_ids={str(x.get('id','')) for x in s.get('providers',[]) if x.get('id')}
                    clean=[]
                    for item in incoming:
                        if not isinstance(item,dict) or not item.get('name') or not item.get('base_url') or not item.get('model'): continue
                        pid=str(item.get('id') or 'provider-'+str(int(time.time()*1000)))
                        old=next((x for x in s.get('providers',[]) if x.get('id')==pid),{})
                        models=item.get('models') if isinstance(item.get('models'),list) else [item['model']]
                        models=list(dict.fromkeys(str(x).strip() for x in models if str(x).strip()))
                        names=item.get('model_names') if isinstance(item.get('model_names'),dict) else {}
                        names={str(k):str(v).strip() for k,v in names.items() if str(v).strip()}
                        clean.append({'id':pid,'name':str(item['name']).strip(),'base_url':str(item['base_url']).strip().rstrip('/'),'model':str(item.get('model') or models[0]).strip(),'models':models,'model_names':names})
                        if 'api_key' in item and item['api_key'].strip(): set_provider_key(pid,item['api_key'].strip())
                    for removed_id in previous_ids-{x['id'] for x in clean}: set_provider_key(removed_id,'')
                    s['providers']=clean
                models=d.get('models',s.get('models',[]))
                if isinstance(models,list): s['models']=list(dict.fromkeys(str(x).strip() for x in models if str(x).strip()))
                if d.get('model'): s['model']=d['model'].strip()
                if d.get('language') in ('zh-CN','en'): s['language']=d['language']
                if isinstance(d.get('default_chat_model'),str): s['default_chat_model']=d['default_chat_model'].strip()
                theme=d.get('theme')
                if isinstance(theme,dict):
                    current=s.get('theme',{})
                    for name in ('accent','bg','card'):
                        value=str(theme.get(name,'')).strip()
                        if len(value)==7 and value.startswith('#'): current[name]=value
                    preset=str(theme.get('preset','custom'))
                    if preset in ('winglight','wingdark','midnight','forest','sunset','custom'): current['preset']=preset
                    s['theme']=current
                if s.get('model') and s['model'] not in s['models']: s['models'].append(s['model'])
                save_settings(s); return self.send_json({'ok':True,'nvidia_configured':bool(get_api_key()),**s})
            if path=='/api/chats':
                chats=d.get('chats',[])
                if not isinstance(chats,list): return self.send_json({'error':'聊天数据格式错误'},400)
                CHATS.write_text(json.dumps(chats,ensure_ascii=False,indent=2)); return self.send_json({'ok':True})
            if path in ('/api/git-stage','/api/git-unstage','/api/git-checkout','/api/git-pull'):
                p=next((x for x in items if x['id']==d.get('id')),None)
                if not p: return self.send_json({'error':'项目不存在'},404)
                try: cwd=project_git_cwd(p,d.get('scope','frontend'))
                except RuntimeError as e: return self.send_json({'error':str(e)},400)
                if path in ('/api/git-stage','/api/git-unstage'):
                    files=d.get('files',[])
                    if not isinstance(files,list) or not files: return self.send_json({'error':'请选择文件'},400)
                    files=[str(x) for x in files]
                    if path=='/api/git-stage': r=run_args(['git','add','--',*files],cwd)
                    else:
                        r=run_args(['git','restore','--staged','--',*files],cwd)
                        if r['code']!=0: r=run_args(['git','reset','HEAD','--',*files],cwd)
                elif path=='/api/git-checkout':
                    branch=str(d.get('branch',''))
                    if branch not in git_branches(cwd): return self.send_json({'error':'分支不存在'},400)
                    r=run_args(['git','switch',branch],cwd)
                else:
                    branch=str(d.get('branch',''))
                    if branch not in git_branches(cwd): return self.send_json({'error':'分支不存在'},400)
                    current=run_args(['git','branch','--show-current'],cwd)['stdout'].strip()
                    if current!=branch:
                        switched=run_args(['git','switch',branch],cwd)
                        if switched['code']!=0: return self.send_json(switched,400)
                    r=run_args(['git','pull','--ff-only'],cwd,120)
                return self.send_json(r,200 if r['code']==0 else 400)
            if path=='/api/ai-analyze':
                key=get_api_key()
                if not key: return self.send_json({'error':'请先在设置中配置 NVIDIA API Key'},400)
                text=d.get('text','').strip()
                if not text: return self.send_json({'error':'没有可分析的日志或报错'},400)
                messages=[{'role':'user','content':'请分析下面的服务日志或报错，按“问题原因 / 影响 / 建议修复”三段简洁回答，使用中文。\n\n'+text[-16000:]}]
                provider_id=d.get('provider_id','nvidia'); selected=d.get('model','').strip(); provider,key,selected=provider_config(provider_id,selected)
                if not key: return self.send_json({'error':'请先在设置中配置该供应商的 API Key'},400)
                base=provider.get('base_url','https://integrate.api.nvidia.com/v1').rstrip('/')
                payload=json.dumps({'model':selected,'messages':messages,'temperature':0.2}).encode(); req=Request(base+'/chat/completions',data=payload,headers={'Authorization':'Bearer '+key,'Content-Type':'application/json'})
                try:
                    out=json.loads(urlopen(req,timeout=60).read()); answer=out['choices'][0]['message']['content'].strip(); return self.send_json({'analysis':answer})
                except Exception as e: return self.send_json({'error':str(e)},500)
            pid=d.get('id'); p=next((x for x in items if x['id']==pid),None)
            if not p: return self.send_json({'error':'项目不存在'},404)
            if path in ('/api/start','/api/stop','/api/restart'):
                key=d.get('service'); action=path.split('/')[-1]
                if key not in ('frontend','backend'): return self.send_json({'error':'无效服务类型'},400)
                stop_result={'ok':True}
                if action in ('stop','restart'): stop_result=stop_service(p,key)
                if not stop_result.get('ok'): result=stop_result
                elif action in ('start','restart'): result=start_service(p,key)
                else: result=stop_result
                save(items); return self.send_json({'project':project_view(p),**result})
            if path=='/api/frontend-build':
                mode=d.get('mode')
                if mode not in ('production','test'): return self.send_json({'error':'无效构建类型'},400)
                task=f'{pid}:{mode}'
                with BUILD_TASKS_LOCK:
                    if BUILD_TASKS.get(task,{}).get('status') in ('starting','running'): return self.send_json({'error':'该项目正在构建'},400)
                    BUILD_TASKS[task]={'status':'starting'}
                threading.Thread(target=run_frontend_build,args=(dict(p),mode),daemon=True).start()
                return self.send_json({'ok':True})
            if path=='/api/delete':
                items=[x for x in items if x['id']!=pid]; save(items); return self.send_json({'ok':True})
            if path=='/api/commit':
                try: cwd=project_git_cwd(p,d.get('scope','frontend'))
                except RuntimeError as e: return self.send_json({'error':str(e)},400)
                msg=d.get('message','').strip();
                if not msg: return self.send_json({'error':'提交信息不能为空'},400)
                if not any(x['staged'] for x in git_files(cwd)): return self.send_json({'error':'没有已暂存的文件'},400)
                r=run_args(['git','commit','-m',msg],cwd,600)
                if r['code']==124: r['error']='提交检查运行超过 10 分钟，请在终端检查 Husky 或 lint-staged 任务'
                if r['code']==0:
                    commit=run_args(['git','rev-parse','HEAD'],cwd,15)['stdout'].strip()
                    r['commit']=commit; r['short_commit']=commit[:8]; r['commit_url']=git_commit_url(cwd,commit)
                return self.send_json(r,200 if r['code']==0 else 400)
            if path=='/api/push':
                try: cwd=project_git_cwd(p,d.get('scope','frontend'))
                except RuntimeError as e: return self.send_json({'error':str(e)},400)
                branch=run_args(['git','branch','--show-current'],cwd,15)['stdout'].strip()
                if not branch: return self.send_json({'error':'当前处于 detached HEAD，无法自动推送'},400)
                upstream=run_args(['git','rev-parse','--abbrev-ref','--symbolic-full-name','@{u}'],cwd,15)
                args=['git','push'] if upstream['code']==0 else ['git','push','-u','origin',branch]
                r=run_args(args,cwd,600)
                if r['code']==124: r['error']='推送运行超过 10 分钟，请检查网络或 Git 凭据'
                r['branch']=branch; r['upstream_created']=upstream['code']!=0
                return self.send_json(r,200 if r['code']==0 else 400)
            if path=='/api/ai-message':
                scope=d.get('scope','frontend')
                try: cwd=project_git_cwd(p,scope)
                except RuntimeError as e: return self.send_json({'error':str(e)},400)
                settings=load_settings(); selected=settings.get('default_chat_model',''); default_pid=selected.partition('::')[0]
                default_provider=next((x for x in settings.get('providers',[]) if x.get('id')==default_pid),None) or next(iter(settings.get('providers',[])),{})
                if not default_provider: return self.send_json({'error':'请先在设置中添加模型平台'},400)
                if not provider_key(default_provider.get('id','')): return self.send_json({'error':'默认模型平台尚未配置 API Key'},400)
                if not run('git diff --cached',cwd,30)['stdout'].strip(): return self.send_json({'error':'请先暂存需要提交的文件'},400)
                task_id=pid+':'+scope
                with AI_TASKS_LOCK:
                    current=AI_TASKS.get(task_id,{})
                    if current.get('status')!='running':
                        AI_TASKS[task_id]={'status':'running','started_at':time.time()}
                        threading.Thread(target=generate_commit_message,args=(task_id,cwd),daemon=True).start()
                return self.send_json({'task_id':task_id,'status':'running'})
        self.send_json({'error':'未知操作'},404)

if __name__=='__main__':
    port=int(os.environ.get('VIBEWING_PORT','8765'))
    print(f'VibeWing running at http://127.0.0.1:{port}',flush=True)
    ThreadingHTTPServer(('127.0.0.1',port),Handler).serve_forever()
