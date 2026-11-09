import fs from 'node:fs';

/** 逐页对账：组件里用到的 class 是否都在对应 .mp-* 旧版样式块里被定义过。
 *  目的：抓「自己造了个旧版没有的类」这种偏差（旧版样式块不会给它任何声明）。 */
const PAIRS = [
  ['mp-pin-group', ['app/member/pin/group/page.tsx', 'src/components/PinGroupTime.tsx']],
  ['mp-comment-list', ['app/member/comment/page.tsx', 'src/components/CommentCenterClient.tsx']],
  ['mp-comment-card', ['src/components/MemberCommentList.tsx']],
  ['mp-comment-info', ['app/member/comment/info/page.tsx', 'src/components/CommentInfoClient.tsx']],
  ['mp-orderinvoice-list', ['app/member/orderinvoice/page.tsx', 'src/components/OrderInvoiceClient.tsx']],
  ['mp-userinvoice-info', ['app/member/userinvoice/info/page.tsx', 'src/components/InvoiceQualificationClient.tsx']],
  ['mp-userinvoice-confirm', ['app/member/userinvoice/confirminfo/page.tsx']],
  ['mp-claim-info', ['app/member/order/claiminfo/page.tsx', 'src/components/OfflineClaimClient.tsx']],
];

const scss = fs.readFileSync('app/member-pages.scss', 'utf8').split(/\r?\n/);

function block(slug) {
  const start = scss.findIndex((l) => l.startsWith('.' + slug + ' {') || l.startsWith('.' + slug + '{'));
  if (start < 0) return null;
  let depth = 0, out = [];
  for (let i = start; i < scss.length; i++) {
    const line = scss[i];
    out.push(line);
    for (const ch of line) {
      if (ch === '{') depth++;
      else if (ch === '}') depth--;
    }
    if (depth === 0 && i > start) break;
  }
  return out.join('\n');
}

function definedClasses(text) {
  const set = new Set();
  text.replace(/\.(-?[_a-zA-Z][\w-]*)/g, (_, name) => { set.add(name); return _; });
  return set;
}

/** 已知「结构性 / 通用」类：来自旧版 components（MemberNav、TigButton、Empty 等）和
 *  element-plus / semi 内部命名，不在本页 <style> 里定义，属预期。 */
const ALLOW = new Set([
  'el-form-item', 'el-form-item__label', 'el-form-item__content', 'el-form', 'el-input',
  'el-input__wrapper', 'el-input__inner', 'el-input__suffix', 'el-textarea', 'el-textarea__inner',
  'el-button', 'el-checkbox', 'el-checkbox__input', 'el-checkbox__inner', 'el-checkbox__label',
  'el-checkbox__original', 'el-radio', 'el-radio-group', 'el-radio__input', 'el-radio__inner',
  'el-radio__label', 'el-radio__original', 'el-upload', 'el-upload--picture-card', 'el-upload__trigger',
  'el-upload-list', 'el-upload-list--picture-card', 'el-upload-list__item',
  'el-upload-list__item-thumbnail', 'el-upload-list__item-actions', 'el-upload-list__item-delete',
  'el-page', 'el-pagination', 'el-pagination__sizes', 'el-pagination__editor', 'el-pagination__total',
  'el-dialog', 'el-dialog__header', 'el-dialog__title', 'el-dialog__body', 'el-dialog__footer',
  'el-rate', 'el-rate__item', 'el-rate__icon', 'el-rate__text', 'el-empty', 'el-empty__description',
  'el-select', 'el-select__wrapper', 'el-option', 'el-drawer', 'el-image', 'el-image__inner',
  'tig-input', 'tig-button', 'tig-tabs', 'theme-button', 'big-button', 'small', 'active', 'tab',
  'no-result', 'sign', 'sign1', 'empty-img', 'empty-text', 'font-color', 'cup', 'ml10', 'mt10', 'mt20',
  'flex', 'lr', 'is-background', 'btn-prev', 'btn-next', 'number', 'current', 'is-active', 'is-checked',
  'is-disabled', 'is-limit-reached', 'mp-el-form--auto',
]);

let bad = 0;
for (const [slug, files] of PAIRS) {
  const text = block(slug);
  if (!text) { console.log(`!! ${slug} 不在 member-pages.scss 里`); bad++; continue; }
  const defined = definedClasses(text);
  const used = new Set();
  let missingFile = false;
  for (const f of files) {
    if (!fs.existsSync(f)) { console.log(`!! 缺文件 ${f}`); missingFile = true; continue; }
    const src = fs.readFileSync(f, 'utf8');
    for (const m of src.matchAll(/className=(?:"([^"]*)"|\{([\s\S]*?)\})/g)) {
      const raw = (m[1] || m[2] || '').replace(/\$\{[^}]*\}/g, ' ');
      for (const token of raw.split(/[\s'"`+]+/)) {
        if (/^[a-z][\w-]*$/i.test(token) && token.length > 1) used.add(token);
      }
    }
  }
  const orphans = [...used].filter((c) => !defined.has(c) && !ALLOW.has(c) && !c.startsWith('semi') && !c.startsWith('ant-'));
  console.log(`\n== ${slug}  (块内定义 ${defined.size} 个类 / 组件用到 ${used.size} 个)`);
  console.log(orphans.length ? '   未在本页旧版样式里定义: ' + orphans.sort().join(', ') : '   全部命中');
  bad += orphans.length;
}
console.log(`\n合计可疑 ${bad} 个`);
