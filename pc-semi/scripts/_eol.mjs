#!/usr/bin/env node
/** 报告文件的换行风格（CRLF / LF / 混合），并给出 CRLF 行数示例。 */
import fs from 'node:fs';

for (const f of process.argv.slice(2)) {
  const buf = fs.readFileSync(f, 'utf8');
  const crlf = (buf.match(/\r\n/g) || []).length;
  const lf = (buf.match(/(?<!\r)\n/g) || []).length;
  console.log(`${f}: CRLF=${crlf} LF=${lf} → ${crlf && lf ? '混合' : crlf ? 'CRLF' : 'LF'}`);
}
