import fs from 'node:fs';
const file = 'src/decorate/components/modules.tsx';
let text = fs.readFileSync(file, 'utf8');
const log = [];
const apply = (from, to, label) => {
  if (!text.includes(from)) {
    log.push(`跳过（没找到）：${label}`);
    return;
  }
  text = text.replace(from, to);
  log.push(`已改：${label}`);
};

// 1) 拼团列表容器：旧版 .mod_groupon > .groupon_list_wrap > .groupon_list
apply(
  "            <div className={`goods-ad-warp${style === 6 ? ' has-nav' : ''}`}",
  "            <div className={`goods-ad-warp groupon_list_wrap${style === 6 ? ' has-nav' : ''}`}",
  'groupon_list_wrap',
);
apply(
  "                className={`goods-ad-con${style === 6 ? ' is-swiper' : ''}`}",
  "                className={`goods-ad-con groupon_list${style === 6 ? ' is-swiper' : ''}`}",
  'groupon_list',
);

// 2) 商品分组购物车图标：旧版 .dec-product-group__cart-icon 包住图标
apply(
  "                            <span className=\"dec-product-group__cart-inner\">",
  "                            <span className=\"dec-product-group__cart-inner dec-product-group__cart-icon\">",
  'dec-product-group__cart-icon',
);

// 3) 分割线：旧版 PcDivider.vue 的 .line-style
apply(
  '          className="dec-divider__line"',
  '          className="dec-divider__line line-style"',
  'line-style',
);

fs.writeFileSync(file, text, 'utf8');
console.log(log.join('\n'));
