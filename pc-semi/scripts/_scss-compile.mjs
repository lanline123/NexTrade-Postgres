#!/usr/bin/env node
/**
 * 用项目里装的 dart-sass 编译单个 scss，确认语法没问题（不跑 next build）。
 * 用法：node scripts/_scss-compile.mjs app/core-pages.scss
 */
import * as sass from 'sass';
import { pathToFileURL } from 'node:url';

const file = (process.argv[2] || '').replace(/^["']+|["']+$/g, '');
try {
  const r = sass.compile(file, { style: 'compressed', logger: sass.Logger.silent });
  console.log(`OK ${file} -> ${r.css.length} bytes css`);
} catch (e) {
  console.log(`FAIL ${file}`);
  console.log(String(e.message).slice(0, 4000));
  process.exitCode = 1;
}
void pathToFileURL;
