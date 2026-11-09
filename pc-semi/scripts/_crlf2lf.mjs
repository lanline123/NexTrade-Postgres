#!/usr/bin/env node
/** 把文件的 CRLF 统一成 LF（就地），用于修 build_file_edit 多行匹配失败的问题。 */
import fs from 'node:fs';

for (const f of process.argv.slice(2)) {
  const before = fs.readFileSync(f, 'utf8');
  const after = before.replace(/\r\n/g, '\n');
  if (before === after) {
    console.log(`跳过（已是 LF）: ${f}`);
    continue;
  }
  fs.writeFileSync(f, after, 'utf8');
  console.log(`已转 LF: ${f}  (${(before.match(/\r\n/g) || []).length} 行 CRLF)`);
}
