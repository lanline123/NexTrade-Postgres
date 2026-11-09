#!/usr/bin/env node
/** 删掉 scripts/_legacy-slice.mjs 生成的临时目录（可随时重建，不该留在仓库里）。 */
import fs from 'node:fs';

for (const dir of ['.tmp-legacy']) {
  if (fs.existsSync(dir)) {
    fs.rmSync(dir, { recursive: true, force: true });
    console.log(`removed ${dir}`);
  } else {
    console.log(`${dir} 不存在`);
  }
}
