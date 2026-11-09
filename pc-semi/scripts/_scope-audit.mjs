// 检查生成的群 SCSS 是否有「顶层选择器没被 .pg-* 包住」的泄漏：node scripts/_scope-audit.mjs app/shop-page.scss ...
import fs from 'node:fs';

for (const file of process.argv.slice(2)) {
  const text = fs.readFileSync(file, 'utf8');
  const lines = text.split(/\r?\n/);
  let depth = 0;
  const offenders = [];
  let inComment = false;
  lines.forEach((raw, index) => {
    const line = raw.replace(/\/\*.*?\*\//g, '').trim();
    if (!line) return;
    if (line.startsWith('/*')) {
      inComment = true;
      return;
    }
    if (inComment) {
      if (line.includes('*/')) inComment = false;
      return;
    }
    const opens = (line.match(/{/g) || []).length;
    const closes = (line.match(/}/g) || []).length;
    if (depth === 0 && opens > 0 && line.endsWith('{')) {
      const head = line.slice(0, -1).trim();
      if (!head.startsWith('.pg-')) offenders.push(`${index + 1}: ${head.slice(0, 90)}`);
    }
    depth += opens - closes;
    if (depth < 0) {
      offenders.push(`${index + 1}: depth went negative`);
      depth = 0;
    }
  });
  console.log(`${file}: balance=${depth} top-level-without-pg=${offenders.length}`);
  offenders.slice(0, 12).forEach((row) => console.log('   ' + row));
}
