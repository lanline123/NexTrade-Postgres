// Windows 上 Next 14 的 build 会同时「清理 distDir」与「异步 createWriteStream 写 distDir/trace」，
// 竞态导致 EPERM: open '<distDir>/trace'。官方没有禁用本地 trace 写入的开关，
// 这里给 to-json.js 加一个环境变量短路（HANDOVER §12.5 记过同一个坑）。
// 幂等：重装依赖后由 prebuild / predev 自动重打。
import fs from 'node:fs';
import path from 'node:path';

const target = path.resolve('node_modules/next/dist/trace/report/to-json.js');
const marker = 'if (process.env.PC_SKIP_BUILD_TRACE) return;';
const needle = 'const reportToLocalHost = (event)=>{';

if (!fs.existsSync(target)) {
    console.log('[patch-next-trace] next 未安装，跳过');
    process.exit(0);
}

const source = fs.readFileSync(target, 'utf8');
if (source.includes(marker)) {
    console.log('[patch-next-trace] 已打过补丁');
    process.exit(0);
}
if (!source.includes(needle)) {
    console.error('[patch-next-trace] 未找到插入点，Next 版本可能已变化');
    process.exit(1);
}

fs.writeFileSync(target, source.replace(needle, `${needle}\n    ${marker}`), 'utf8');
console.log('[patch-next-trace] 已写入 trace 短路');
