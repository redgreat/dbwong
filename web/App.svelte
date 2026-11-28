<script>
  import { onMount } from 'svelte';
  import ResourceList from './ResourceList.svelte';
  import Modal from './Modal.svelte';
  import './admin.css';

  const admin = location.pathname.startsWith('/admin');
  const detailId = Number(location.pathname.match(/^\/admin\/connections\/(\d+)$/)?.[1]);
  const menus = { connections: '数据库连接', accounts: '账号管理', access: '读取授权', settings: '会话设置', audit: '操作审计' };
  const actions = {
    connection: '新增连接', edit_connection: '编辑连接', delete_connection: '删除连接',
    account: '新增账号', account_status: '修改账号状态', reset_password: '重置密码',
    grant: '新增授权', revoke: '撤销授权', session_hours: '编辑会话设置'
  };
  const labels = {
    'account.create':'创建账号','account.enable':'恢复账号','account.disable':'停用账号',
    'account.reset_password':'重置密码','connection.create':'登记连接','connection.edit':'修改连接',
    'connection.delete':'删除连接','read.grant':'授予读取','read.revoke':'撤销读取',
    'settings.session_hours':'修改会话时长','probe.start':'开始探测','probe.ok':'探测成功',
    'probe.failed':'探测失败','probe.timeout':'探测超时'
  };
  const dateFormat = new Intl.DateTimeFormat('zh-CN', { dateStyle:'medium', timeStyle:'medium' });
  let view = $state('connections');
  let query = $state('');
  let data = $state(null);
  let dark = $state(false);
  let ready = $state(false);
  let message = $state('');
  let modalError = $state('');
  let modal = $state(null);
  let busy = $state(false);
  let now = $state(new Date());
  let events = $state([]);
  let nextBefore = $state(null);
  let auditLoading = $state(false);
  let auditError = $state('');
  const environment = (value) => value === 'production' ? '生产' : '测试';
  const status = (enabled) => enabled ? '启用' : '停用';
  const rows = (items, term) => items.filter(row => row.cells.some(cell => String(cell).toLowerCase().includes(term.trim().toLowerCase())));
  const connectionRows = $derived((data?.connections ?? []).map(row => ({
    ...row, cells: [row.name, row.engine, environment(row.environment), ...(admin ? [status(row.enabled)] : [])]
  })));
  const selected = $derived(connectionRows.find(row => row.id === detailId));
  const accountRows = $derived((data?.accounts ?? []).map(row => ({
    ...row, cells:[row.name, row.admin ? '管理员' : 'DBA', status(row.enabled)]
  })));
  const grantRows = $derived((data?.grants ?? []).map(([account, connection]) => ({
    account, connection, cells:[accountRows.find(row => row.id === account)?.name ?? account,
      connectionRows.find(row => row.id === connection)?.name ?? connection]
  })));

  async function api(path, body) {
    const response = await fetch(path, body === undefined ? {cache:'no-store'} : {
      method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
    });
    if (!response.ok) {
      const error = new Error(({400:'请检查表单内容',401:'请使用有效账号和密码登录',403:'当前账号没有此权限',404:'记录已不存在',409:'名称已存在或记录状态不可用'})[response.status] ?? '服务暂不可用');
      error.status = response.status;
      throw error;
    }
    return response.headers.get('content-type')?.includes('application/json') ? response.json() : null;
  }
  async function refresh() {
    const result = await api(admin ? '/api/admin' : '/api/connections');
    data = admin ? result : {connections: result};
  }
  async function loadAudit(append = false) {
    if (auditLoading) return;
    auditLoading = true;
    auditError = '';
    try {
      const page = await api('/api/admin/audit' + (append ? '?before=' + nextBefore : ''));
      events = append ? [...events, ...page.events] : page.events;
      nextBefore = page.next_before;
    } catch (error) { auditError = error.message; }
    finally { auditLoading = false; }
  }
  function navigate(next) {
    if (detailId) { location.assign('/admin#' + next); return; }
    view = next;
    query = '';
    message = '';
    history.replaceState(null, '', '/admin#' + next);
    if (next === 'audit') void loadAudit();
  }
  function toggleTheme() {
    dark = !dark;
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    localStorage.setItem('dbwong-theme', document.documentElement.dataset.theme);
  }
  function open(action, row = null) { modalError = ''; modal = {action, row}; }
  function close() { if (!busy) modal = null; }
  async function login(event) {
    event.preventDefault();
    busy = true;
    try {
      await api('/api/login', Object.fromEntries(new FormData(event.currentTarget)));
      await refresh();
      message = '';
      if (view === 'audit') void loadAudit();
    } catch (error) { message = error.message; }
    finally { busy = false; }
  }
  async function save(event) {
    event.preventDefault();
    if (busy) return;
    busy = true;
    modalError = '';
    const fields = Object.fromEntries(new FormData(event.currentTarget));
    const {action, row} = modal;
    const payload = { ...fields, action: action === 'revoke' ? 'grant' : action };
    if (action === 'edit_connection' || action === 'delete_connection') payload.id = row.id;
    if (action === 'edit_connection') payload.enabled = fields.enabled === 'yes';
    if (action === 'account_status' || action === 'reset_password') payload.account = row.id;
    if (action === 'account_status') payload.enabled = !row.enabled;
    if (action === 'grant' || action === 'revoke') {
      payload.account = Number(row?.account ?? fields.account);
      payload.connection = Number(row?.connection ?? fields.connection);
      payload.allow = action === 'grant';
    }
    if (action === 'session_hours') payload.hours = Number(fields.hours);
    try {
      await api('/api/admin', payload);
      modal = null;
      message = '操作已保存';
      await refresh();
    } catch (error) {
      if (modal) modalError = error.message;
      else message = '已保存，刷新列表失败。请重新加载页面。';
    } finally { busy = false; }
  }
  async function probe(row) {
    busy = true;
    try {
      const result = await api('/api/admin/probe', {connection:row.id});
      message = ({'probe.ok':'已连接，查询成功','probe.failed':'连接或认证失败','probe.timeout':'连接测试超时'})[result.outcome];
    } catch (error) { message = error.message; }
    finally { busy = false; }
  }
  async function logout() {
    try { await api('/api/logout', {}); location.reload(); }
    catch (error) { message = error.message; }
  }
  onMount(() => {
    dark = localStorage.getItem('dbwong-theme') === 'dark';
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
    const hash = location.hash.slice(1);
    if (Object.hasOwn(menus, hash)) view = hash;
    refresh().then(() => { if (admin && view === 'audit') void loadAudit(); })
      .catch(error => { if (error.status !== 401) message = error.message; })
      .finally(() => { ready = true; });
    const timer = setInterval(() => { now = new Date(); }, 1000);
    return () => clearInterval(timer);
  });
</script>

<svelte:head><title>{admin ? '管理后台' : '工作台'} · dbwong</title></svelte:head>
<header>
  <strong><span class="mark" aria-hidden="true">D</span>dbwong</strong>
  <a href="/" aria-current={!admin ? 'page' : undefined}>工作台</a>
  <a href="/admin" aria-current={admin ? 'page' : undefined}>管理后台</a>
  <button type="button" onclick={toggleTheme}>{dark ? '浅色' : '深色'}</button>
  {#if data}<button type="button" onclick={logout}>退出</button>{/if}
</header>

<div class="app-shell" class:admin-active={admin && !!data} class:signin={ready && !data}>
  {#if admin && data}
    <aside><nav aria-label="后台菜单">
      {#each Object.entries(menus) as [key, name]}
        <button type="button" aria-pressed={view === key} onclick={() => navigate(key)}>{name}</button>
      {/each}
    </nav></aside>
  {/if}
  <main>
    <h1>{!ready ? '正在加载…' : !data ? (admin ? '管理后台登录' : '登录工作台') : !admin ? '我的工作台' : detailId ? '连接详情' : menus[view]}</h1>
    {#if message}<p id="message" role="status">{message}</p>{/if}
    {#if ready && !data}
      <form id="login" onsubmit={login}>
        <label>账号<input name="name" required autocomplete="username"></label>
        <label>密码<input name="password" type="password" required autocomplete="current-password"></label>
        <button disabled={busy}>登录</button>
      </form>
    {:else if data}
      <div id="content">
        {#if detailId}
          <a class="secondary" href="/admin#connections">返回连接列表</a>
          {#if selected}
            <section class="detail">
              <div class="panel-heading"><h2>{selected.name}</h2><button onclick={() => open('edit_connection', selected)}>编辑</button></div>
              <dl><dt>编号</dt><dd>{selected.id}</dd><dt>引擎</dt><dd>{selected.engine}</dd><dt>环境</dt><dd>{environment(selected.environment)}</dd><dt>状态</dt><dd>{status(selected.enabled)}</dd></dl>
              {#if selected.engine === 'postgres'}
                <button class="secondary" disabled={!selected.enabled || busy} onclick={() => probe(selected)}>测试连接</button>
              {/if}
            </section>
          {:else}<section class="detail">此连接不存在或已删除。</section>{/if}
        {:else if view === 'connections' || !admin}
          <ResourceList title="数据库连接" bind:query rows={rows(connectionRows, query)} columns={admin ? ['名称','引擎','环境','状态'] : ['名称','引擎','环境']}
            actions={admin ? connectionActions : undefined} create={admin ? () => open('connection') : undefined} />
        {:else if view === 'accounts'}
          <ResourceList title="账号" bind:query rows={rows(accountRows, query)} columns={['账号','角色','状态']} actions={accountActions} create={() => open('account')} />
        {:else if view === 'access'}
          <ResourceList title="授权" bind:query rows={rows(grantRows, query)} columns={['账号','连接']} actions={grantActions} create={() => open('grant')} />
        {:else if view === 'settings'}
          <ResourceList title="会话设置" bind:query rows={rows([{cells:['新登录会话有效期', data.session_hours + ' 小时']}], query)} columns={['设置项','当前值']} actions={settingsActions} />
        {:else if view === 'audit'}
          <ResourceList title="已加载审计记录" bind:query
            rows={rows(events.map(event => ({cells:[dateFormat.format(new Date(event.occurred_at)), event.actor, labels[event.action] ?? event.action, event.subject, event.connection ?? '—']})), query)}
            columns={['时间','操作人','操作','对象编号','连接']} extra={auditNotice} more={auditControls} empty={auditLoading ? '正在加载…' : '暂无匹配记录'} />
        {/if}
      </div>
    {/if}
  </main>
  <footer><span>Copyright © wangcw {now.getFullYear()} · dbwong</span><time datetime={now.toISOString()}>{dateFormat.format(now)}</time></footer>
</div>

{#snippet connectionActions(row)}
  <a class="secondary" href={'/admin/connections/' + row.id}>详情</a>
  <button class="secondary" onclick={() => open('edit_connection', row)}>编辑</button>
  <button class="secondary destructive" onclick={() => open('delete_connection', row)}>删除</button>
{/snippet}
{#snippet accountActions(row)}
  {#if !row.admin}
    <button class="secondary" onclick={() => open('account_status', row)}>{row.enabled ? '停用' : '恢复'}</button>
    <button class="secondary" onclick={() => open('reset_password', row)}>重置密码</button>
  {/if}
{/snippet}
{#snippet grantActions(row)}
  <button class="secondary destructive" onclick={() => open('revoke', row)}>撤销</button>
{/snippet}
{#snippet settingsActions(_row)}
  <button class="secondary" onclick={() => open('session_hours')}>编辑</button>
{/snippet}
{#snippet auditNotice()}
  {#if auditError}<p role="alert">{auditError}</p>{/if}
{/snippet}
{#snippet auditControls()}
  <div class="row-actions">
    <button class="secondary" disabled={auditLoading} onclick={() => loadAudit()}>刷新</button>
    {#if nextBefore}<button class="secondary" disabled={auditLoading} onclick={() => loadAudit(true)}>加载更多</button>{/if}
  </div>
{/snippet}

{#if modal}
  <Modal title={actions[modal.action]} {close}>
    <form onsubmit={save} autocomplete="off">
      {#if modal.action === 'connection' || modal.action === 'edit_connection'}
        <label>名称<input name="name" required maxlength="100" value={modal.row?.name ?? ''}></label>
        {#if modal.action === 'connection'}
          <label>引擎<select name="engine">{#each ['mysql','postgres','mssql','mongo','redis'] as engine}<option>{engine}</option>{/each}</select></label>
        {/if}
        <label>环境<select name="environment" value={modal.row?.environment ?? 'test'}><option value="test">测试</option><option value="production">生产</option></select></label>
        {#if modal.action === 'edit_connection'}
          <label>状态<select name="enabled" value={modal.row.enabled ? 'yes' : 'no'}><option value="yes">启用</option><option value="no">停用</option></select></label>
        {/if}
        <label>{modal.action === 'connection' ? '连接串' : '新连接串（留空沿用）'}<input name="dsn" type="password" required={modal.action === 'connection'} autocomplete="new-password"></label>
      {:else if modal.action === 'account' || modal.action === 'reset_password'}
        {#if modal.action === 'account'}<label>账号<input name="name" required maxlength="100"></label>{:else}<p>重置 {modal.row.name} 的密码会结束该账号的现有会话。</p>{/if}
        <label>新密码（12–72 字节）<input name="password" type="password" required autocomplete="new-password"></label>
      {:else if modal.action === 'grant'}
        <label>账号<select name="account" required><option value="">选择账号</option>{#each accountRows as row}<option value={row.id}>{row.name}</option>{/each}</select></label>
        <label>连接<select name="connection" required><option value="">选择连接</option>{#each connectionRows as row}<option value={row.id}>{row.name}</option>{/each}</select></label>
      {:else if modal.action === 'session_hours'}
        <label>新登录会话有效期（小时）<input name="hours" type="number" min="1" max="168" value={data.session_hours} required></label>
      {:else if modal.action === 'delete_connection'}
        <p>删除「{modal.row.name}」将清除保存的凭据并撤销现有授权，审计记录继续保留。</p>
      {:else if modal.action === 'revoke'}
        <p>确认撤销「{modal.row.cells[0]}」对「{modal.row.cells[1]}」的读取授权？</p>
      {:else if modal.action === 'account_status'}
        <p>{modal.row.enabled ? '停用并结束当前会话' : '恢复登录'}：{modal.row.name}</p>
      {/if}
      {#if modalError}<p role="alert">{modalError}</p>{/if}
      <div class="row-actions"><button class="secondary" type="button" disabled={busy} onclick={close}>取消</button><button disabled={busy}>{modal.action === 'delete_connection' ? '确认删除' : '确认'}</button></div>
    </form>
  </Modal>
{/if}
