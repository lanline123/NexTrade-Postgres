// 打印首页装修返回的模块 type 列表（只读，不改任何东西）。
// 用法: node scripts/_home-types.mjs [previewId]
const BASE = (process.env.PC_API_BASE || 'https://battery.recy.cc').replace(/\/$/, '');
const previewId = Number(process.argv[2] || 0) || 0;
const url = `${BASE}/api/home/home/pcIndex${previewId ? `?previewId=${previewId}` : ''}`;
const res = await fetch(url, { headers: { accept: 'application/json' } });
const json = await res.json();
const data = json?.data ?? json;
const list = data?.moduleList || [];
console.log('HTTP', res.status, 'modules', list.length, 'homeStyleVersion', data?.pageModule?.homeStyleVersion);
list.forEach((item, i) => {
  const m = item?.module || {};
  const keys = Object.keys(m).slice(0, 8).join(',');
  console.log(
    String(i + 1).padStart(2, ' '),
    String(item?.type || '?').padEnd(22),
    'isShow=' + item?.isShow,
    'idx=' + item?.moduleIndex,
    'title=' + JSON.stringify(m.title?.titleText ?? m.title ?? '').slice(0, 40),
    '| ' + keys,
  );
});
