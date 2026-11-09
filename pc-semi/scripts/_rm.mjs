// 递归删除给定路径（脚本自用；仅接受项目内相对路径）。
// 用法: node scripts/_rm.mjs <path> [path...]
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
for (const raw of process.argv.slice(2)) {
  const p = String(raw).replace(/^["']+|["']+$/g, '');
  const abs = path.resolve(root, p);
  if (!abs.startsWith(root)) {
    console.error(`拒绝：${p} 不在项目内`);
    continue;
  }
  if (!fs.existsSync(abs)) {
    console.log(`跳过（不存在）：${p}`);
    continue;
  }
  fs.rmSync(abs, { recursive: true, force: true });
  console.log(`已删除：${p}`);
}
