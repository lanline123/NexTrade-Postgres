import fs from 'node:fs';
import path from 'node:path';
const rm = (p) => {
  if (!fs.existsSync(p)) {
    console.log(`跳过（不存在）：${p}`);
    return;
  }
  fs.rmSync(p, { recursive: true, force: true });
  console.log(`已删：${p}`);
};
rm('.tmp-audit');
rm('docs/_round21.md');
rm('docs/_round22.md');
rm('scripts/_patch-modules-r22a.mjs');
rm('scripts/_patch-modules-groupon2.mjs');
rm('scripts/_fix-groupon-close.mjs');
rm('scripts/_patch-groupon-span.mjs');
rm('scripts/_patch-modules-carticon.mjs');
console.log('保留：gen-*.mjs 生成器、_patch-modules-hero/groupon/seckill/countdown.mjs（可重放的记录）');
