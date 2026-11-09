#!/usr/bin/env node
/**
 * 一次性：把搜索页的分类横条换成旧版 Filter.vue 结构（含左右箭头、.item 内层），
 * 并去掉重复的 .filter-category__/category-link 手写块。
 */
import fs from 'node:fs';

const file = 'app/search/page.tsx';
let s = fs.readFileSync(file, 'utf8');
const before = s.length;

// 1) import
if (!s.includes("from '@/components/SearchCategoryStrip'")) {
  s = s.replace(
    "import SearchSortBar from '@/components/SearchSortBar';",
    "import SearchSortBar from '@/components/SearchSortBar';\nimport SearchCategoryStrip from '@/components/SearchCategoryStrip';"
  );
}

// 2) 分类横条：旧版 Filter.vue 的 .attr-box 里放箭头 + 滚动容器
const oldBlock = `                      <div className="attr-box">
                        <div className="category-link-wrapper">
                          <div className="category-link">
                            <Link href={tabHref('goods')} className={\`item\${cat ? '' : ' active'}\`}>
                              <span className="category-name">{t('全部')}</span>
                            </Link>
                            {categories.map((item) => {
                              const id = String(item.id ?? '');
                              return (
                                <Link
                                  key={id}
                                  href={\`\${listPath}\${buildQuery({ cat: id }, 1)}\`}
                                  className={\`item\${cat === id ? ' active' : ''}\`}
                                >
                                  <span className="category-name">{String(item.name ?? '')}</span>
                                </Link>
                              );
                            })}
                          </div>
                        </div>
                      </div>`;

const newBlock = `                      <div className="attr-box">
                        <SearchCategoryStrip items={categoryStripItems} />
                      </div>`;

if (!s.includes(oldBlock)) {
  console.error('分类横条旧块没找到，未替换');
} else {
  s = s.replace(oldBlock, newBlock);
}

// 3) 供上面用的 items（旧版「全部」永远第一项）
s = s.replace(
  '  /* 旧版品牌下拉：点一项就切到该品牌，再点一次取消 */',
  `  /* 旧版 Filter.vue 一级分类：第一项固定是「全部」 */
  const categoryStripItems = [
    { key: 'all', label: t('全部'), href: tabHref('goods'), active: !cat },
    ...categories.map((item) => {
      const id = String(item.id ?? '');
      return {
        key: id || String(item.name ?? ''),
        label: String(item.name ?? ''),
        href: \`\${listPath}\${buildQuery({ cat: id }, 1)}\`,
        active: cat === id,
      };
    }),
  ];

  /* 旧版品牌下拉：点一项就切到该品牌，再点一次取消 */`
);

fs.writeFileSync(file, s, 'utf8');
console.log(`${file}: ${before} -> ${s.length} 字节`);
console.log(s.includes('SearchCategoryStrip items') ? '分类横条已接线' : '分类横条未接线');
