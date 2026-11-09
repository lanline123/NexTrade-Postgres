import fs from 'node:fs';

/** 删除一段「从某行开始、到某行之前」的代码：node scripts/_cut-range.mjs spec.json
 *  spec = { file, from: "唯一子串", to: "唯一子串" }  → 删掉 [from 所在行, to 所在行) 之间的所有行
 *  用于清理大段死代码，避开 build_file_edit 的 CRLF/多行限制。 */
const spec = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const raw = fs.readFileSync(spec.file, 'utf8');
const eol = raw.includes('\r\n') ? '\r\n' : '\n';
const lines = raw.split(/\r?\n/);

const hits = (needle) => lines.reduce((acc, l, i) => (l.includes(needle) ? [...acc, i] : acc), []);
const fromHits = hits(spec.from);
const toHits = hits(spec.to);
if (fromHits.length !== 1) throw new Error(`from 命中 ${fromHits.length} 次，需唯一: ${spec.from}`);
if (toHits.length !== 1) throw new Error(`to 命中 ${toHits.length} 次，需唯一: ${spec.to}`);
const a = fromHits[0];
const b = toHits[0];
if (b <= a) throw new Error('to 必须在 from 之后');
const removed = b - a;
const out = [...lines.slice(0, a), ...lines.slice(b)];
fs.writeFileSync(spec.file, out.join(eol));
console.log(`${spec.file}: 删除第 ${a + 1}–${b} 行（${removed} 行），${lines.length} → ${out.length} 行`);
