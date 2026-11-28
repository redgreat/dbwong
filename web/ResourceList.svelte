<script>
  let { title, rows, columns, actions, create, query = $bindable(''), extra, more, empty = '暂无记录' } = $props();
</script>

<section class="resource">
  <div class="panel-heading">
    <label class="query">查询<input bind:value={query} placeholder={'搜索' + title}></label>
    {#if create}<button type="button" onclick={create}>新增</button>{/if}
  </div>
  {@render extra?.()}
  <div class="table-scroll">
    <table aria-label={title}>
      <thead><tr>{#each columns as column}<th scope="col">{column}</th>{/each}{#if actions}<th scope="col">操作</th>{/if}</tr></thead>
      <tbody>
        {#each rows as row}
          <tr>{#each row.cells as cell}<td>{cell}</td>{/each}{#if actions}<td><div class="row-actions">{@render actions(row)}</div></td>{/if}</tr>
        {:else}
          <tr><td class="empty" colspan={columns.length + Number(Boolean(actions))}>{empty}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {@render more?.()}
</section>
