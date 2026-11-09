const res = await fetch('http://127.0.0.1:3004/api/product/product/list?page=1&size=8&productSelectType=0');
const json = await res.json();
const records = json?.data?.records || [];
console.log(records.map((r) => r.productId).join(','));
console.log(records.map((r) => `${r.productId}:${r.picUrl ? 'pic' : 'nopic'}`).join(' '));
