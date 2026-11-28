<script>

import {onMount} from 'svelte';
import './admin.css';
let dark=$state(false);
let events=$state([]);
let nextBefore=$state(null);
let auditLoading=$state(false);
let auditError=$state('');
const auditLabels={
 'account.create':'创建账号','account.enable':'恢复账号','account.disable':'停用账号',
 'account.reset_password':'重置密码','connection.create':'登记连接','connection.edit':'修改连接',
 'read.grant':'授予读取','read.revoke':'撤销读取','settings.session_hours':'修改会话时长',
 'probe.start':'开始探测','probe.ok':'探测成功','probe.failed':'探测失败','probe.timeout':'探测超时'
};
async function loadAudit(append=false){
 if(auditLoading)return;
 auditLoading=true;
 auditError='';
 try{
   const result=await api('/api/admin/audit'+(append?'?before='+nextBefore:''));
   events=append?[...events,...result.events]:result.events;
   nextBefore=result.next_before;
 }catch(error){
   auditError=error.message;
 }finally{
   auditLoading=false;
 }
}
function toggleTheme(){
 dark=!dark;
 document.documentElement.dataset.theme=dark?'dark':'light';
 localStorage.setItem('dbwong-theme',document.documentElement.dataset.theme);
}
const el=id=>document.getElementById(id);
const admin=location.pathname.startsWith('/admin');
const heading=admin?'管理后台':'我的工作台';
let title=$state(heading);
async function api(path,body){
 const options=body===undefined?{}:{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(body)};
 const response=await fetch(path,{...options,cache:'no-store'});
 if(!response.ok)throw Object.assign(new Error(({400:'请检查表单内容',401:'请使用有效账号和密码登录',403:'当前账号没有此权限',409:'名称已存在'})[response.status]||'服务暂不可用'),{status:response.status});
 return response.headers.get('content-type')?.includes('application/json')?response.json():null;
}
function list(id,values){el(id).replaceChildren(...values.map(text=>{const li=document.createElement('li');li.textContent=text;return li}))}
function options(name,rows){el('grant').elements[name].replaceChildren(...rows.map(r=>new Option(r.name,r.id)))}
async function refresh(){
 el('content').hidden=true;
 try{
 const data=await api(admin?'/api/admin':'/api/connections');
 const rows=admin?data.connections:data;
 list('connections',rows.length?rows.map(r=>r.name+' · '+r.engine+' · '+(r.environment==='production'?'生产':'测试')):['暂无连接']);
 if(admin){el('session_hours').elements.hours.value=data.session_hours;options('account',data.accounts);options('connection',rows);
 for(const id of ['reset_password','account_status']) {
   const choices=data.accounts.filter(a=>!a.admin).map(a=>new Option(a.name+(a.enabled?'':'（已停用）'),a.id));
   el(id).elements.account.replaceChildren(...choices)
 }
 const edit=el('edit_connection').elements;
 edit.target.replaceChildren(...rows.map(r=>new Option(r.name+(r.enabled?'':'（已停用）'),r.id)));
 function selectConnection(){
   const row=rows.find(r=>r.id===Number(edit.target.value));
   edit.name.value=row?.name??'';edit.dsn.value='';edit.environment.value=row?.environment??'test';
   edit.enabled.value=row?.enabled?'yes':'no';
   el('probe').disabled=!row?.enabled||row.engine!=='postgres';
 }
 edit.target.onchange=selectConnection;selectConnection();
 const names=new Map(data.accounts.map(r=>[r.id,r.name])), labels=new Map(rows.map(r=>[r.id,r.name]));
 list('grants',data.grants.map(([a,c])=>names.get(a)+' → '+labels.get(c)))}
 el('login').hidden=true;el('content').hidden=false;el('sidebar').hidden=!admin;el('dashboard').classList.toggle('dashboard',admin);
 if(!admin){el('connection').hidden=true;el('edit_connection').hidden=true}
 document.body.classList.remove('signin');document.body.classList.toggle('admin-active',admin);title=heading;el('logout').hidden=false;
 }catch(e){document.body.classList.remove('admin-active');document.body.classList.add('signin');title=admin?'管理后台登录':'登录工作台';el('login').hidden=false;el('logout').hidden=e.status===401;el('message').textContent=e.status===401?'':e.message}
}
async function submit(event){
 const form=event.currentTarget;
 event.preventDefault();const data=Object.fromEntries(new FormData(form));const signin=form.getAttribute('id')==='login';
 if(form.getAttribute('id')==='account_status'||form.getAttribute('id')==='reset_password')data.account=Number(data.account);
 if(form.getAttribute('id')==='account_status')data.enabled=data.enabled==='yes';
 if(form.getAttribute('id')==='edit_connection'){data.id=Number(data.target);delete data.target;data.enabled=data.enabled==='yes'}
 if(form.getAttribute('id')==='session_hours') data.hours=Number(data.hours);
 if(form.getAttribute('id')==='grant'){data.account=Number(data.account);data.connection=Number(data.connection);data.allow=data.allow==='yes'}
 try{await api(signin?'/api/login':'/api/admin',signin?data:{action:form.getAttribute('id'),...data});form.reset();el('message').textContent=signin?'':'已保存';await refresh()}catch(e){el('message').textContent=e.message}
};
async function probe(){
 const connection=Number(el('edit_connection').elements.target.value);
 try{const result=await api('/api/admin/probe',{connection});
 el('message').textContent=({'probe.ok':'已连接，查询成功','probe.failed':'连接或认证失败','probe.timeout':'连接测试超时'})[result.outcome]}
 catch(e){el('message').textContent=e.message}
};
async function logout(){try{await api('/api/logout',{});location.reload()}catch(e){el('message').textContent=e.message}};
onMount(()=>{
 dark=localStorage.getItem('dbwong-theme')==='dark';
 document.documentElement.dataset.theme=dark?'dark':'light';
title=heading;
document.title=heading+' · dbwong';
document.querySelector(admin?'header a[href="/admin"]':'header a[href="/"]').setAttribute('aria-current','page');
const panels=[...document.querySelectorAll('[data-panel]')];
const menu=[...document.querySelectorAll('[data-view]')];
for(const button of menu) button.onclick=()=>{
 for(const item of menu) item.setAttribute('aria-pressed',String(item===button));
 for(const panel of panels) panel.hidden=panel.dataset.panel!==button.dataset.view;
 el('message').textContent='';
 if(button.dataset.view==='audit')void loadAudit();
};
 refresh();
});
</script>
<header><strong><span class="mark" aria-hidden="true">D</span>dbwong</strong><a href="/">工作台</a><a href="/admin">管理后台</a><button type="button" onclick={toggleTheme}>{dark ? "浅色" : "深色"}</button><button id="logout" onclick={logout} hidden>退出</button></header>
<main><h1>{title}</h1><p id="message" role="status"></p>
<form onsubmit={submit} id="login" hidden><h2>登录账号</h2><label>账号<input name="name" required autocomplete="username"></label><label>密码<input name="password" type="password" required autocomplete="current-password"></label><button>登录</button></form>
<div id="content" hidden><div id="dashboard">
<aside id="sidebar" hidden><nav aria-label="后台菜单">
<button type="button" data-view="connections" aria-pressed="true"><svg viewBox="0 0 16 16" aria-hidden="true"><ellipse cx="8" cy="3.5" rx="5" ry="2"/><path d="M3 3.5v9c0 2.6 10 2.6 10 0v-9M3 8c0 2.6 10 2.6 10 0"/></svg>数据库连接</button>
<button type="button" data-view="accounts" aria-pressed="false"><svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="5" r="2.5"/><path d="M3 14v-1a5 5 0 0 1 10 0v1"/></svg>账号管理</button>
<button type="button" data-view="access" aria-pressed="false"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 1.5 14 4v4c0 3-3 5.5-6 7-3-1.5-6-4-6-7V4zM5.5 8l1.6 1.6 3.4-3.4"/></svg>读取授权</button>
<button type="button" data-view="settings" aria-pressed="false"><svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6"/><path d="M8 4.5V8l2.5 1.5"/></svg>会话设置</button>
<button type="button" data-view="audit" aria-pressed="false">操作审计</button>
</nav></aside>
<div id="workspace"><div data-panel="connections"><section><h2>数据库连接</h2><ul id="connections"></ul></section>
<form onsubmit={submit} id="connection"><h2>登记连接</h2><label>名称<input name="name" required></label><label>引擎<select name="engine"><option>mysql</option><option>postgres</option><option>mssql</option><option>mongo</option><option>redis</option></select></label><label>环境<select name="environment"><option value="test">测试</option><option value="production">生产</option></select></label><label>连接串<input name="dsn" type="password" required autocomplete="off"></label><button>保存</button></form>
<form onsubmit={submit} id="edit_connection"><h2>维护已登记连接</h2><label>连接<select name="target" required></select></label><label>名称<input name="name" required></label><label>环境<select name="environment"><option value="test">测试</option><option value="production">生产</option></select></label><label>状态<select name="enabled"><option value="yes">启用</option><option value="no">停用</option></select></label><label>新连接串（留空沿用）<input name="dsn" type="password" autocomplete="off"></label><button>保存修改</button><button type="button" id="probe" onclick={probe}>测试已保存的 PostgreSQL 连接</button></form>
</div><div data-panel="accounts" hidden><form onsubmit={submit} id="account"><h2>创建 DBA 账号</h2><label>账号<input name="name" required></label><label>密码（12–72 字节）<input name="password" type="password" required autocomplete="new-password"></label><button>创建</button></form>
<form onsubmit={submit} id="account_status"><h2>账号状态</h2><label>DBA 账号<select name="account" required></select></label><label>操作<select name="enabled"><option value="no">停用并结束会话</option><option value="yes">恢复登录</option></select></label><button>应用</button></form>
<form onsubmit={submit} id="reset_password"><h2>重置 DBA 密码</h2><label>DBA 账号<select name="account" required></select></label><label>新密码（12–72 字节）<input name="password" type="password" autocomplete="new-password" required></label><button>重置并结束会话</button></form>
</div><div data-panel="settings" hidden><form onsubmit={submit} id="session_hours"><h2>会话设置</h2><label>新登录会话有效期（小时）<input name="hours" type="number" min="1" max="168" required></label><button>保存</button></form></div><div data-panel="access" hidden><form onsubmit={submit} id="grant"><h2>读取授权</h2><label>账号<select name="account" required></select></label><label>连接<select name="connection" required></select></label><label>操作<select name="allow"><option value="yes">授予读取</option><option value="no">撤销读取</option></select></label><button>应用</button><ul id="grants"></ul></form></div>
<div data-panel="audit" hidden>
 <section>
  <div class="panel-heading"><h2>操作审计</h2><button type="button" disabled={auditLoading} onclick={() => loadAudit()}>刷新</button></div>
  {#if auditError}<p role="alert">{auditError}</p>{/if}
  {#if events.length}
    <div class="table-scroll">
     <table aria-label="管理操作记录">
      <thead><tr><th>时间</th><th>操作人</th><th>操作</th><th>对象编号</th><th>连接</th></tr></thead>
      <tbody>
       {#each events as event (event.id)}
        <tr><td>{new Date(event.occurred_at).toLocaleString()}</td><td>{event.actor}</td><td>{auditLabels[event.action] ?? event.action}</td><td>{event.subject}</td><td>{event.connection ?? '—'}</td></tr>
       {/each}
      </tbody>
     </table>
    </div>
    {#if nextBefore}<button type="button" disabled={auditLoading} onclick={() => loadAudit(true)}>加载更多</button>{/if}
  {:else if !auditError}<p>{auditLoading ? '正在加载…' : '暂无操作记录'}</p>{/if}
 </section>
</div></div></div></div></main>
