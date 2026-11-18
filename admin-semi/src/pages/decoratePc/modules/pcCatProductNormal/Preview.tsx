/**
 * PC 分类商品-窄（pcCatProductNormal）- 预览组件
 *
 * 渲染规格对齐前台 pc-semi ContractCatProductNormal
 * （pc-semi/src/decorate/components/modules.tsx:4458，逐字复刻旧版
 * components/home/CatProductNormal.vue）：
 *   .mod_columns_floor（608×430）
 *     标题栏：linear-gradient(30deg, color, color2) 白字渐变条 + 标题 + 子分类关键词（前 6，竖线分隔）
 *     左：picList 淡入轮播（5000ms、悬停暂停、圆点色 = color），caption 用 color 着色
 *     右：picList2 促销卡（全部渲染壳，仅前 4 张有内容），副标题用 color 着色
 *   旧版该模块不渲染商品列表（只在接口声明里出现），故此处不拉商品接口。
 *
 * 与前台差异：
 * 1. 子分类关键词：前台消费后端富化的 module.childCategoryList；编辑器数据里
 *    带 childCategoryList 时优先用它，否则按 categoryId 从分类树
 *    （/product/category/getAllCategory）取子分类兜底；
 * 2. 链接一律 javascript:; 不跳转；标题箭头 iconfont-pc 字体编辑器不可用，改内联 SVG；
 * 3. picList 为空时渲染一张占位图保持左栏尺寸（前台为空白盒），缺图走内联 SVG 占位。
 */
import { useEffect, useMemo, useState } from 'react';
import { apiGet } from '../../../../api';
import { useI18n } from '../../../../i18n';
import './pcCatProductNormal.css';

type AnyRecord = Record<string, any>;

const EMPTY_IMG =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='100' height='100'%3E%3Crect fill='%23f2f4f6' width='100' height='100'/%3E%3Cpath d='M28 62l13-16 10 11 7-7 14 12H28z' fill='%23c0c8d4'/%3E%3Ccircle cx='40' cy='38' r='6' fill='%23c0c8d4'/%3E%3C/svg%3E";

/** 与 def defaults 逐字段一致（modules/pcCatProductNormal/index.tsx）。 */
const DEFAULTS: AnyRecord = {
  categoryId: 0,
  title: '',
  shortTitle: '',
  enTitle: '',
  color: '',
  color2: '',
  logo: { picUrl: '', picThumb: '' },
  picList: [],
  picList2: [],
};

function isPlainObject(value: unknown): value is AnyRecord {
  return Object.prototype.toString.call(value) === '[object Object]';
}

function deepMerge<T extends AnyRecord>(defaults: T, source?: AnyRecord | null): T {
  const out: AnyRecord = Array.isArray(defaults) ? [...defaults] : { ...defaults };
  if (!source) return out as T;
  Object.keys(source).forEach((key) => {
    const base = out[key];
    const next = source[key];
    if (isPlainObject(base) && isPlainObject(next)) {
      out[key] = deepMerge(base as AnyRecord, next as AnyRecord);
    } else if (next !== undefined) {
      out[key] = next;
    }
  });
  return out as T;
}

function picSrc(item?: AnyRecord | null): string {
  if (!item) return EMPTY_IMG;
  return item.picUrl || item.picThumb || EMPTY_IMG;
}

function titleTextOf(raw: any): string {
  return typeof raw === 'string' ? raw : (raw?.titleText as string) || '';
}

/* ---------- 分类树（子分类关键词兜底取数，与 PcDecorateEditor useCategoryTree 同口径） ---------- */

let categoryTreeCache: AnyRecord[] | null = null;

function findChildren(tree: AnyRecord[], categoryId: number): AnyRecord[] {
  for (const node of tree) {
    if (Number(node.categoryId) === categoryId) {
      return Array.isArray(node.children) ? node.children : [];
    }
    if (Array.isArray(node.children)) {
      const found = findChildren(node.children, categoryId);
      if (found.length) return found;
    }
  }
  return [];
}

function useCategoryChildren(categoryId: number): AnyRecord[] {
  const [children, setChildren] = useState<AnyRecord[]>([]);
  useEffect(() => {
    if (!categoryId) {
      setChildren([]);
      return undefined;
    }
    let alive = true;
    const apply = (tree: AnyRecord[]) => {
      if (alive) setChildren(findChildren(tree, categoryId));
    };
    if (categoryTreeCache) {
      apply(categoryTreeCache);
    } else {
      apiGet<any[]>('/product/category/getAllCategory')
        .then((raw) => {
          categoryTreeCache = Array.isArray(raw) ? raw : [];
          apply(categoryTreeCache);
        })
        .catch(() => {
          if (alive) setChildren([]);
        });
    }
    return () => {
      alive = false;
    };
  }, [categoryId]);
  return children;
}

/** 标题右侧圆角箭头（前台 iconfont-pc icon-yuanjiaojiantouyou 的等价内联 SVG）。 */
function TitleArrow() {
  return (
    <svg className="dec-pc-cat-normal__tit-arrow" width="14" height="14" viewBox="0 0 1024 1024" aria-hidden="true">
      <path
        d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm193.5 456.9-192 128c-12.9 8.6-30.9 5-39.5-7.9-8.6-12.9-5.1-30.3 7.9-39.5l141.6-94.4-141.6-94.4c-12.9-8.6-16.5-26.6-7.9-39.5 8.6-12.9 26.6-16.5 39.5-7.9l192 128c7.8 5.2 12.5 13.9 12.5 23.1s-4.7 17.9-12.5 23.1z"
        fill="currentColor"
      />
    </svg>
  );
}

export default function PcCatProductNormalPreview({ data }: { data: Record<string, any> }) {
  useI18n();
  const config = useMemo(() => deepMerge(DEFAULTS, data), [JSON.stringify(data)]);
  const titleText = titleTextOf(config.title);
  const color = typeof config.color === 'string' ? config.color : '';
  const color2 = typeof config.color2 === 'string' ? config.color2 : '';
  const picList: AnyRecord[] = Array.isArray(config.picList) ? config.picList : [];
  const picList2: AnyRecord[] = Array.isArray(config.picList2) ? config.picList2 : [];
  const categoryId = Number(Array.isArray(config.categoryId) ? config.categoryId[0] : config.categoryId) || 0;

  // 子分类关键词：存量 childCategoryList 优先，否则按 categoryId 取分类树子级
  const storedChildren: AnyRecord[] = Array.isArray(config.childCategoryList) ? config.childCategoryList : [];
  const treeChildren = useCategoryChildren(storedChildren.length ? 0 : categoryId);
  const childCategories = (storedChildren.length ? storedChildren : treeChildren).slice(0, 6);

  // 左侧轮播（前台 useCarousel：5000ms 淡入、悬停暂停）
  const [index, setIndex] = useState(0);
  const [paused, setPaused] = useState(false);
  useEffect(() => {
    setIndex(0);
  }, [picList.length]);
  useEffect(() => {
    if (paused || picList.length <= 1) return undefined;
    const timer = window.setInterval(() => {
      setIndex((prev) => (prev + 1) % picList.length);
    }, 5000);
    return () => window.clearInterval(timer);
  }, [paused, picList.length]);

  const titStyle = color || color2
    ? { background: `linear-gradient(30deg, ${color || color2} 0%, ${color2 || color} 100%)` }
    : undefined;
  const textColorStyle = color ? { color } : undefined;
  const bulletStyle = color ? { backgroundColor: color } : undefined;

  return (
    <div className="dec-pc-cat-normal">
      <div className="dec-pc-cat-normal__floor">
        <div className="dec-pc-cat-normal__tit" style={titStyle}>
          <h4 className="dec-pc-cat-normal__tit-name">
            <a href="javascript:;" title={titleText}>
              {titleText}
            </a>
            <TitleArrow />
          </h4>
          <div className="dec-pc-cat-normal__keywords">
            <ul>
              {childCategories.map((item, i) => (
                <li key={String(item?.categoryId ?? i)}>
                  <a href="javascript:;" title={item?.categoryName || ''}>
                    {item?.categoryName || ''}
                  </a>
                  {' | '}
                </li>
              ))}
            </ul>
          </div>
        </div>
        <div className="dec-pc-cat-normal__row">
          {picList.length ? (
            <div
              className="dec-pc-cat-normal__slider"
              onMouseEnter={() => setPaused(true)}
              onMouseLeave={() => setPaused(false)}
            >
              {picList.map((item, i) => (
                <div
                  key={i}
                  className="dec-pc-cat-normal__slide"
                  style={{ opacity: i === index ? 1 : 0, zIndex: i === index ? 2 : 1 }}
                >
                  <a href="javascript:;" title={item?.picTitle || ''}>
                    <p className="dec-pc-cat-normal__caption" style={textColorStyle}>
                      {item?.picTitle || ''}
                    </p>
                    <p className="dec-pc-cat-normal__sub">{item?.picDesc || ''}</p>
                    <span className="dec-pc-cat-normal__slide-img">
                      <img src={picSrc(item)} alt={item?.picTitle || ''} />
                    </span>
                  </a>
                </div>
              ))}
              <div className="dec-pc-cat-normal__dots">
                {picList.map((_, i) => (
                  <span
                    key={i}
                    className={`dec-pc-cat-normal__dot${i === index ? ' is-active' : ''}`}
                    style={bulletStyle}
                    onClick={() => setIndex(i)}
                  />
                ))}
              </div>
            </div>
          ) : (
            <div className="dec-pc-cat-normal__slider">
              <div className="dec-pc-cat-normal__slide" style={{ opacity: 1 }}>
                <span className="dec-pc-cat-normal__slide-img">
                  <img src={EMPTY_IMG} alt="" />
                </span>
              </div>
            </div>
          )}
          <div className="dec-pc-cat-normal__pro-con">
            {picList2.map((item, i) => (
              <div className="dec-pc-cat-normal__pro-wrap" key={i}>
                {i < 4 ? (
                  <a href="javascript:;" title={item?.picTitle || ''}>
                    <p className="dec-pc-cat-normal__pro-tit">{item?.picTitle || ''}</p>
                    <p className="dec-pc-cat-normal__pro-sub" style={textColorStyle}>
                      {item?.picDesc || ''}
                    </p>
                    <span className="dec-pc-cat-normal__pro-img">
                      <img src={picSrc(item)} alt={item?.picTitle || ''} />
                    </span>
                  </a>
                ) : null}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
