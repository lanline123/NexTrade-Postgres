const url = 'http://127.0.0.1:3004/api/product/product/list?page=1&categoryId=1&size=8';
const res = await fetch(url);
const json = await res.json().catch(() => null);
const records = json?.data?.records || json?.data || [];
console.log(`${res.status} records=${Array.isArray(records) ? records.length : 'n/a'}`);
console.log(JSON.stringify(json).slice(0, 300));
