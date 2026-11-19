/**
 * PC 首页装修编辑器（对齐原版 5.8.61 新版模块集 modules/pcHome，/decorate/pc 独立编辑器）
 *
 * 数据契约（与原版逐字段一致，透传不裁剪）：
 * - data = { pageModule, moduleList }
 * - pageModule = { type:'page', module:{}, backgroundRepeat:'', backgroundSize:'', style:0,
 *                  headerStyle:1, homeStyleVersion:'new'|'old', pcHeader?(仅新版) }
 * - moduleList 项 = { moduleIndex: Date.now(), type, url:'pc', label, isShow, module:{...defaults} }
 * - 新套 20 模块 kebab-case（pc-home-hero / pc-image-ad / ...），旧套 16 模块 camelCase 仅兼容历史数据
 * - 加载: GET decorate/decorate/detail?id&decorateType=2&localeId=0&parentId=0
 * - 保存: POST decorate/decorate/saveDraft | publish  body {id, decorateType:2, pageType:1, localeId:0, parentId:0, data}
 */
import { Component, useEffect, useRef, useState, type CSSProperties, type ErrorInfo, type ReactNode } from 'react';
import {
  Button,
  ColorPicker,
  Input,
  InputNumber,
  Modal,
  Pagination,
  Popconfirm,
  Popover,
  Radio,
  RadioGroup,
  Select,
  SideSheet,
  Slider,
  Space,
  Spin,
  Switch,
  Tag,
  TextArea,
  Toast,
  Tooltip,
  Typography,
} from '@douyinfe/semi-ui';
import {
  IconArrowLeft,
  IconCart,
  IconChevronDown,
  IconChevronUp,
  IconClose,
  IconCopy,
  IconDelete,
  IconEyeClosed,
  IconEyeOpened,
  IconGlobe,
  IconHandle,
  IconUser,
  IconPlus,
  IconRefresh,
} from '@douyinfe/semi-icons';
import { apiGet, apiPost } from '../api';
import { loadMallSettings, resolveMallUrl } from '../mall';
import i18nInstance, { useI18n } from '../i18n';

/** 供模块级工具函数 / 字段定义使用的翻译（非组件作用域拿不到 useI18n，用 i18next 单例）。 */
const tl = (key: string, params?: Record<string, string | number>) =>
  i18nInstance.t(key, params) as string;
import DecorateLoading from './components/DecorateLoading';
import ProductPickGrid, { mergePickedItems, type ProductPickItem } from '../components/ProductPickGrid';
import ProductPickerModal from '../components/ProductPickerModal';
import { PcBuyButtonVisual, resolvePcBuyBtn, usePcGlobalBuyStyle, SEARCH_LEGACY_MAP } from './decoratePc/previews/shared/PcBuyButton';
import AlphaColorInput from '../components/AlphaColorInput';
import ImageUploader from '../components/ImageUploader';
import ProductImagePickerModal from '../components/ProductImagePicker';
import type { PickedImage } from '../components/ProductImagePicker';
import GalleryPickerModal from '../components/GalleryPickerModal';
import LinkPickerModal, { LinkField } from '../components/LinkPicker';
import SliderWithInput from '../components/SliderWithInput';
import SharedColorField, { ColorPairField } from '../components/ColorField';
import { PC_LIB_META as LIB_META } from '../components/editorModuleIcons';
import EditorShell from '../components/decorate/EditorShell';
import LibraryPanel from '../components/decorate/LibraryPanel';
import DecSettingsPanel from '../components/decorate/DecSettingsPanel';
import ViewGridList from '@icon-park/react/es/icons/ViewGridList';
import Local from '@icon-park/react/es/icons/Local';
import './decorate-editor.css';
/* 固定标题共享排版令牌：各 pc*.css 只引用变量，数值只在这里定义一次 */
import './decoratePc/previews/shared/module-title.shared.css';

const { Text } = Typography;

/* ===================== 共享默认值构造（原版 _shared/types.ts 逐字段） ===================== */

import { allPcModuleDefs, getPcModuleDef, PC_MODULE_PREVIEWS } from './decoratePc/modules';
import { PC_LOGIN_DEFAULTS } from './decoratePc/modules/pcLogin';
import { PC_SEARCH_RESULT_DEFAULTS } from './decoratePc/modules/pcSearchResult';
import {
  BUY_BTN_STYLE_OPTIONS, DEMO_PIC_1, DEMO_PIC_2, HERO_CATEGORIES, HERO_DEMO_PRODUCTS, NOTICE_ICON, QUICK_NAV_ITEMS,
  buyBtnFields, containerFields, demoPic, fieldHint, getPath, heroDemoModules, normalizeLegacyBuyBtn, normalizeLegacyTitle,
  productsDefaults, setPath, standardFrame, standardTitle, titleFields,
  type ModuleDef, type PicItem, type SettingFieldDef,
} from './decoratePc/contract/pcModuleContract';

/* 模块定义自 Step 3 起拆至 decoratePc/modules/<type>/ 自注册，此处按原顺序从注册表取回（数据格式不变）。 */
const NEW_MODULE_DEFS: ModuleDef[] = [
  'pc-banner2', 'pc-group-banner', 'pc-image-ad', 'pc-image-nav', 'pc-image-square-ad', 'pc-image-hotarea',
  'pc-home-quick-nav', 'pc-category-nav', 'pc-category-nav3', 'pc-product', 'pc-product-group', 'pc-recommend-product',
  'pc-home-coupon', 'pc-home-seckill', 'pc-home-groupon', 'pc-home-hero', 'pc-notice', 'pc-title-ad', 'pc-divider', 'pc-spacer',
].map((t) => getPcModuleDef(t)!);
const LEGACY_MODULE_DEFS: ModuleDef[] = [
  'pcBanner', 'pcBanner2', 'pcGroupBanner', 'pcImageAd', 'pcCustomAd1', 'pcCustomAd2', 'pcCatProductNormal', 'pcCatProductWide',
  'pcCatProductSimple', 'pcRecommendProduct', 'pcSeckill', 'pcCoupon', 'pcGroupon', 'pcCategoryNav', 'pcCategoryNav3', 'pcCatWide',
].map((t) => getPcModuleDef(t)!);
const ALL_MODULE_DEFS = [...NEW_MODULE_DEFS, ...LEGACY_MODULE_DEFS];
const MODULE_DEF_MAP = new Map(ALL_MODULE_DEFS.map((d) => [d.type, d]));
const PC_LOGIN_MODULE_DEF: ModuleDef = getPcModuleDef('pcLogin')!;
MODULE_DEF_MAP.set('pcLogin', PC_LOGIN_MODULE_DEF);
const PC_SEARCH_RESULT_MODULE_DEF: ModuleDef = getPcModuleDef('pcSearchResult')!;
MODULE_DEF_MAP.set('pcSearchResult', PC_SEARCH_RESULT_MODULE_DEF);
const LEGACY_TYPES = new Set(LEGACY_MODULE_DEFS.map((d) => d.type));

const LIB_GROUP_ORDER = ['商品展示', '图片与轮播', '导航入口', '营销活动', '基础排版', '旧版'];

/* 模块库元数据（排序 order + 图标）已合并到共享总表 components/editorModuleIcons.ts，
   顶部以 PC_LIB_META as LIB_META 导入，与 H5 的 MODULE_ICONPARK 单一源维护。 */

/* ===================== 编辑器组件 ===================== */

interface EditorModule {
  moduleIndex: number;
  type: string;
  url: string;
  label: string;
  isShow: boolean;
  fixed?: boolean;
  module: Record<string, any>;
}

interface PcDecorateEditorProps {
  decorateId: number;
  onExit?: () => void;
}

/** 设置区白卡容器（标准白卡口径）。必须定义在模块级：在组件内定义会使每次渲染
 * 产生新的组件类型，React 会整棵卸载重挂其子树——输入框打字时逐字丢焦点/丢字
 * （v46 验收发现的页面设置/头部设置丢字问题即此）。 */
const SettingsCard = ({ children }: { children: ReactNode }) => (
  <div style={{ background: '#fff', margin: 12, border: '1px solid #eaecf0', borderRadius: 10, padding: 16 }}>{children}</div>
);

export default function PcDecorateEditor({ decorateId, onExit }: PcDecorateEditorProps) {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<'draft' | 'publish' | null>(null);
  const [title, setTitle] = useState('');
  const [pageType, setPageType] = useState(1);
  const [hasDraft, setHasDraft] = useState(false);
  const [canvasScale, setCanvasScale] = useState(1);
  const canvasViewportRef = useRef<HTMLDivElement>(null);
  const [pageModule, setPageModule] = useState<Record<string, any>>({});
  const [moduleList, setModuleList] = useState<EditorModule[]>([]);
  const [activeIndex, setActiveIndex] = useState<number>(-1);
  const [panelTab, setPanelTab] = useState<'module' | 'page' | 'pcHeader'>('module');
  const dirtyRef = useRef(false);
  const decorateIdRef = useRef(decorateId);
  const moduleElRefs = useRef(new Map<number, HTMLElement>());
  const [railTops, setRailTops] = useState<Record<number, number>>({});
  const [railHeights, setRailHeights] = useState<Record<number, number>>({});

  const markDirty = () => {
    dirtyRef.current = true;
  };

  const isLoginPage = pageType === 5;
  const isSearchPage = pageType === 6;

  useEffect(() => {
    void loadMallSettings();
  }, []);

  // 画布主题变量：与 PC 前台同源（后台「主题风格」themeStyle，PC 端 SSR 注入 :root 的同一份数据）。
  // 只注入到 .pc-decorate-window 画布容器（作用域收窄，不影响 admin 自身 UI），
  // 预览里 var(--main-bg) 等主色消费（如商品卡 hover 描边）自动跟随后台设置的 PC 主题色。
  const [themeVars, setThemeVars] = useState<CSSProperties>({});
  useEffect(() => {
    let alive = true;
    apiGet<{ themeStyle?: string | Record<string, any> }>('/setting/config/themeStyleSettings')
      .then((res) => {
        if (!alive) return;
        let parsed: Record<string, any> = {};
        try {
          parsed = typeof res?.themeStyle === 'string' ? JSON.parse(res.themeStyle) : res?.themeStyle || {};
        } catch {
          return;
        }
        const vars: Record<string, string> = {};
        Object.entries(parsed).forEach(([k, v]) => {
          if (!k.startsWith('--')) return;
          if (typeof v !== 'string' && typeof v !== 'number') return;
          const text = String(v).trim();
          if (!text || /[;{}<>\\]/.test(text)) return; // 防 CSS 注入，口径同 pc-semi theme-style.ts
          vars[k] = text;
        });
        setThemeVars(vars as CSSProperties);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);

  // 画布头部地区显示：与前台 PcHeaderTopBar 同口径——不读装修配置（topRegionText 前台已不消费），
  // 按当前访问 IP 经商城接口解析（省份列表命中名优先，其次精简定位名），解析不出显示「全国」。
  const [regionText, setRegionText] = useState('');
  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const base = resolveMallUrl().replace(/\/+$/, '');
        const [provRes, locRes] = await Promise.all([
          fetch(`${base}/api/sys/region/getProvinceList`).then((r) => r.json()).catch(() => null),
          fetch(`${base}/api/sys/region/getLocationByIp`).then((r) => r.json()).catch(() => null),
        ]);
        if (!alive) return;
        const provinces: Array<{ regionId?: number | string; regionName?: string }> = Array.isArray(provRes?.data) ? provRes.data : [];
        const loc = locRes?.data;
        if (loc && loc.regionId !== undefined && loc.regionId !== null) {
          const hit = provinces.find((p) => String(p.regionId) === String(loc.regionId));
          const raw = hit?.regionName || (loc.regionName ? String(loc.regionName) : '');
          const name = raw.replace(/省/g, '').replace(/市/g, '').replace(/自治区/g, '');
          if (name) setRegionText(name);
        }
      } catch {
        /* 解析失败保持空，渲染时回落「全国」 */
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    apiGet<{
      decorateId?: number;
      decorateTitle?: string;
      pageType?: number;
      data?: { pageModule?: Record<string, any>; moduleList?: EditorModule[] } | null;
      draftData?: { pageModule?: Record<string, any>; moduleList?: EditorModule[] } | null;
    }>('/decorate/decorate/detail', { id: decorateId, decorateType: 2, localeId: 0, parentId: 0 })
      .then((res) => {
        if (!alive) return;
        if (res?.decorateId) decorateIdRef.current = res.decorateId;
        setTitle(res?.decorateTitle || '');
        const currentPageType = Number(res?.pageType) || 1;
        setPageType(currentPageType);
        setHasDraft(Boolean(res?.draftData));
        const source = res?.draftData || res?.data || {};
        const pm: Record<string, any> = {
          type: 'page', module: {}, backgroundRepeat: '', backgroundSize: '',
          style: 0, headerStyle: 1,
          ...source.pageModule,
        };
        // 原版 commonPage dealDefault 语义：未设置时落到默认值
        if (pm.backgroundRepeat === undefined || pm.backgroundRepeat === '') pm.backgroundRepeat = 1;
        if (pm.backgroundSize === undefined || pm.backgroundSize === '') pm.backgroundSize = 1;
        if (typeof pm.title !== 'string') pm.title = '';
        // 新版头部装修默认值合并（原版 mergePcHeaderDefaults 语义）
        pm.pcHeader = { ...PC_HEADER_DEFAULTS, ...(pm.pcHeader || {}) };
        if (!pm.homeStyleVersion) pm.homeStyleVersion = 'new';
        setPageModule(pm);
        let list = Array.isArray(source.moduleList) ? source.moduleList : [];
        if (currentPageType === 6) {
          list = list.filter((item: any) => item.type === 'pcSearchResult');
          if (!list.some((item: any) => item.type === 'pcSearchResult')) {
            list = [
              {
                moduleIndex: Date.now(),
                type: 'pcSearchResult',
                url: 'pcSearch',
                label: '搜索结果',
                isShow: true,
                module: { ...PC_SEARCH_RESULT_DEFAULTS },
              },
            ];
          }
        }
        if (currentPageType === 5) {
          list = list.filter((item: any) => item.type === 'pcLogin');
          if (!list.some((item: any) => item.type === 'pcLogin')) {
            list = [
              {
                moduleIndex: Date.now(),
                type: 'pcLogin',
                url: 'pcLogin',
                label: '登录页',
                isShow: true,
                module: { ...PC_LOGIN_DEFAULTS },
              },
            ];
          }
        }
        setModuleList(list.map((item, idx) => ({
          moduleIndex: item.moduleIndex ?? Date.now() + idx,
          type: item.type,
          url: item.url || 'pc',
          label: item.label || MODULE_DEF_MAP.get(item.type)?.label || item.type,
          isShow: item.isShow !== false,
          fixed: item.fixed,
          module: normalizeLegacyBuyBtn(item.type, normalizeLegacyTitle(item.type, item.module || {})),
        })));
      })
      .catch((e) => Toast.error(e?.message || t('加载装修数据失败')))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [decorateId]);

  useEffect(() => {
    const handler = (e: BeforeUnloadEvent) => {
      if (!dirtyRef.current) return;
      e.preventDefault();
      e.returnValue = '';
    };
    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, []);

  // 画布按容器宽度等比缩放，避免 1600px 画布出现横向滚动条
  useEffect(() => {
    const el = canvasViewportRef.current;
    if (!el) return;
    const update = () => {
      const width = el.clientWidth;
      // 为右侧悬浮操作条（56）和左右留白（48）预留空间，避免被设置栏遮住
      const next = Math.min(1, Math.max(0.35, (width - 104) / 1600));
      setCanvasScale(next);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, [loading]);

  // 测量各模块在滚动流中的纵向位置，供左右轨道标签/操作条对齐（原版 railTops 机制）
  useEffect(() => {
    const timer = setTimeout(() => {
      const tops: Record<number, number> = {};
      const heights: Record<number, number> = {};
      moduleElRefs.current.forEach((el, key) => {
        tops[key] = el.offsetTop;
        heights[key] = el.offsetHeight;
      });
      setRailTops(tops);
      setRailHeights(heights);
    }, 80);
    return () => clearTimeout(timer);
  }, [moduleList, activeIndex, loading]);

  const buildPayload = (data: Record<string, any>) => ({
    id: decorateIdRef.current,
    decorateType: 2,
    pageType,
    localeId: 0,
    parentId: 0,
    data,
  });

  const getSaveData = () => {
    const hasLegacy = moduleList.some((m) => LEGACY_TYPES.has(m.type));
    const homeStyleVersion: 'new' | 'old' = hasLegacy ? 'old' : 'new';
    const pm: any = { ...pageModule, homeStyleVersion };
    if (homeStyleVersion !== 'new') {
      delete pm.pcHeader; // 原版契约：旧版不携带头部装修
    }
    return { pageModule: pm, moduleList: moduleList.map((m) => ({ ...m })) };
  };

  const handleSave = async (mode: 'draft' | 'publish') => {
    if (!decorateIdRef.current) {
      Toast.warning(t('缺少装修页面 ID'));
      return;
    }
    setSaving(mode);
    try {
      await apiPost(mode === 'draft' ? '/decorate/decorate/saveDraft' : '/decorate/decorate/publish', buildPayload(getSaveData()));
      Toast.success(mode === 'draft' ? t('草稿已保存') : t('发布成功'));
      dirtyRef.current = false;
    } catch (e: any) {
      Toast.error(e?.message || t('保存失败'));
    } finally {
      setSaving(null);
    }
  };

  const handleRename = async () => {
    if (!title.trim()) {
      Toast.warning(t('页面名称不能为空'));
      return;
    }
    try {
      await apiPost('/decorate/decorate/updateField', { id: decorateIdRef.current, field: 'decorateTitle', val: title.trim() });
      Toast.success(t('页面名称修改成功'));
    } catch (e: any) {
      Toast.error(e?.message || t('修改失败'));
    }
  };

  /** 切换首页装修版本（原版 onSwitchHomeVersion：清空另一套模块，旧版还会清掉头部装修） */
  const switchHomeVersion = (target: 'new' | 'old') => {
    const toNew = target === 'new';
    const clearCount = toNew
      ? moduleList.filter((m) => LEGACY_TYPES.has(m.type)).length
      : moduleList.filter((m) => !LEGACY_TYPES.has(m.type)).length;
    const clearLabel = toNew ? t('旧版') : t('新版');
    const targetLabel = toNew ? t('新版') : t('旧版');
    const clearsHeader = !toNew; // 切旧版时 pcHeader 头部装修数据会随保存一并清空
    const clearParts: string[] = [];
    if (clearCount > 0) clearParts.push(t('{count} 个{mode}模块', { count: clearCount, mode: clearLabel }));
    if (clearsHeader) clearParts.push(t('头部装修数据'));
    Modal.confirm({
      title: t('切换首页装修版本'),
      content: t('切换到{target}后，将清空当前画布上的{parts}；一经预览、保存草稿或发布，将无法恢复。确认切换？', { target: targetLabel, parts: clearParts.join(t('和')) || t('无关内容') }),
      okText: t('确认切换'),
      cancelText: t('取消'),
      onOk: () => {
        setModuleList((prev) => prev.filter((m) => (LEGACY_TYPES.has(m.type) ? !toNew : toNew)));
        setPageModule((prev) => {
          const next: any = { ...prev, homeStyleVersion: target };
          if (target === 'old') delete next.pcHeader;
          return next;
        });
        markDirty();
        Toast.success(t('已切换到{target}首页', { target: targetLabel }));
      },
    });
  };

  const addModule = (def: ModuleDef) => {
    if (isLoginPage) {
      const existing = moduleList.findIndex((m) => m.type === 'pcLogin');
      if (existing >= 0) {
        setActiveIndex(existing);
        setPanelTab('module');
        return;
      }
    }
    if (isSearchPage && def.type === 'pcSearchResult') {
      const existing = moduleList.findIndex((m) => m.type === 'pcSearchResult' && (m.url === 'pcSearch' || !m.url));
      if (existing >= 0) {
        setActiveIndex(existing);
        setPanelTab('module');
        return;
      }
    }
    const item: EditorModule = {
      moduleIndex: Date.now(),
      type: def.type,
      url: def.type === 'pcSearchResult' ? 'pcSearch' : 'pc',
      label: def.label,
      isShow: true,
      module: def.defaults(),
    };
    setModuleList((prev) => [...prev, item]);
    setActiveIndex(moduleList.length);
    setPanelTab('module');
    markDirty();
    Toast.success(t('已添加「{name}」', { name: t(def.label) }));
  };

  /** 拖拽入画布：插入到目标位置（原版 vuedraggable clone 行为） */
  const addModuleAt = (def: ModuleDef, index: number) => {
    if (isLoginPage) {
      addModule(def);
      return;
    }
    const item: EditorModule = {
      moduleIndex: Date.now(),
      type: def.type,
      url: 'pc',
      label: def.label,
      isShow: true,
      module: def.defaults(),
    };
    setModuleList((prev) => [...prev.slice(0, index), item, ...prev.slice(index)]);
    setActiveIndex(index);
    setPanelTab('module');
    markDirty();
    Toast.success(t('已添加「{name}」', { name: t(def.label) }));
  };

  const removeModule = (index: number) => {
    setModuleList((prev) => prev.filter((_, i) => i !== index));
    if (activeIndex === index) setActiveIndex(-1);
    markDirty();
  };

  const copyModule = (index: number) => {
    const clone: EditorModule = { ...JSON.parse(JSON.stringify(moduleList[index])), moduleIndex: Date.now() };
    setModuleList((prev) => [...prev.slice(0, index + 1), clone, ...prev.slice(index + 1)]);
    markDirty();
  };

  const moveModule = (index: number, dir: -1 | 1) => {
    const target = index + dir;
    if (target < 0 || target >= moduleList.length) return;
    setModuleList((prev) => {
      const next = [...prev];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
    if (activeIndex === index) setActiveIndex(target);
    markDirty();
  };

  const toggleShow = (index: number) => {
    setModuleList((prev) => prev.map((m, i) => (i === index ? { ...m, isShow: !m.isShow } : m)));
    markDirty();
  };

  const dropModule = (from: number, to: number) => {
    if (from === to) return;
    setModuleList((prev) => {
      const next = [...prev];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      return next;
    });
    markDirty();
  };

  const updateActiveModule = (updater: (m: Record<string, any>) => Record<string, any>) => {
    setModuleList((prev) => prev.map((m, i) => (i === activeIndex ? { ...m, module: updater(m.module) } : m)));
    markDirty();
  };

  const activeModule = activeIndex >= 0 && activeIndex < moduleList.length ? moduleList[activeIndex] : null;

  /* 模块库分组：新版页按 def.group 五组（LIB_GROUP_ORDER 定序、LIB_META.order 组内排序）；
     旧版语境（页面版本为旧版或画布含旧版模块）只出「旧版」组；登录/搜索页维持单组。 */
  const isLegacyContext = pageModule.homeStyleVersion === 'old' || moduleList.some((m) => LEGACY_TYPES.has(m.type));
  const libSourceDefs = isLoginPage
    ? [PC_LOGIN_MODULE_DEF]
    : isSearchPage
      ? [PC_SEARCH_RESULT_MODULE_DEF]
      : isLegacyContext
        ? LEGACY_MODULE_DEFS
        : NEW_MODULE_DEFS.filter((d) => !d.hiddenInLibrary);
  const toLibItem = (def: ModuleDef) => {
    const LibIcon = LIB_META[def.type]?.Icon || ViewGridList;
    return {
      key: def.type,
      label: t(def.label),
      title: def.note ? t(def.note) : t('{name}（点击添加，可拖入画布指定位置）', { name: t(def.label) }),
      icon: <LibIcon size={28} strokeWidth={3} />,
    };
  };
  const byLibOrder = (a: ModuleDef, b: ModuleDef) => (LIB_META[a.type]?.order ?? 999) - (LIB_META[b.type]?.order ?? 999);
  const libraryGroups = isLoginPage || isSearchPage
    ? [{ key: 'pc-lib', title: isLoginPage ? t('登录页模块') : t('搜索页模块'), items: libSourceDefs.map(toLibItem) }]
    : LIB_GROUP_ORDER.filter((g) => libSourceDefs.some((d) => d.group === g)).map((g) => ({
        key: g,
        title: t(g),
        items: libSourceDefs.filter((d) => d.group === g).sort(byLibOrder).map(toLibItem),
      }));
  const activeDef = activeModule ? MODULE_DEF_MAP.get(activeModule.type) : null;

  const doExit = () => {
    if (onExit) onExit();
    else window.location.href = `${window.location.origin}/?page=pcDecorateManage`;
  };
  const handleExit = () => {
    if (!dirtyRef.current) {
      doExit();
      return;
    }
    Modal.confirm({
      title: t('有未保存的修改'),
      content: t('退出后未保存的修改将丢失，确定退出吗？'),
      okText: t('确定退出'),
      cancelText: t('取消'),
      onOk: doExit,
    });
  };
  if (loading) {
    return <DecorateLoading />;
  }

  return (
    <EditorShell
      className={`pc-decorate-editor${isLoginPage ? ' pc-decorate-editor--login' : ''}`}
      topbar={(
        <>
      <div
        style={{
          display: 'flex', alignItems: 'center', gap: 12, padding: '10px 16px',
          background: '#fff', borderBottom: '1px solid var(--semi-color-border)', flexShrink: 0,
        }}
      >
        <Button
          icon={<IconArrowLeft />}
          onClick={handleExit}
        >
          {t('返回列表')}
        </Button>
        <Space>
          <Button
            icon={<IconRefresh />}
            onClick={() => {
              // 与 AdminLayout 的前台入口保持同一地址来源；不能再打开后台自身的 /?page=home
              // （该 key 不在 knownPages，只会落到「尚未迁移」占位页，属于假预览）。
              window.open(resolveMallUrl(), '_blank');
            }}
          >
            {t('打开商城首页')}
          </Button>
          {!isLoginPage && (
            <>
              <Button theme={panelTab === 'page' ? 'solid' : 'light'} type={panelTab === 'page' ? 'primary' : 'tertiary'} onClick={() => { setPanelTab('page'); setActiveIndex(-1); }}>
                {t('页面设置')}
              </Button>
              <Button theme={panelTab === 'pcHeader' ? 'solid' : 'light'} type={panelTab === 'pcHeader' ? 'primary' : 'tertiary'} onClick={() => { setPanelTab('pcHeader'); setActiveIndex(-1); }}>
                {t('PC头部装修')}
              </Button>
            </>
          )}
        </Space>
        <Space>
          <Input
            value={title}
            onChange={(v) => setTitle(v)}
            onBlur={handleRename}
            onEnterPress={handleRename}
            style={{ width: 280 }}
          />
          {hasDraft && <Tag color="blue">{t('草稿')}</Tag>}
          {dirtyRef.current && <Tag color="orange">{t('未保存')}</Tag>}
          {getSaveData().pageModule.homeStyleVersion === 'new' ? <Tag color="green">{t('新版装修')}</Tag> : <Tag color="grey">{t('旧版兼容')}</Tag>}
        </Space>
        <div style={{ flex: 1 }} />
        <Button theme="light" loading={saving === 'draft'} onClick={() => handleSave('draft')}>
          {t('保存草稿')}
        </Button>
        <Button theme="solid" type="primary" loading={saving === 'publish'} onClick={() => handleSave('publish')}>
          {t('保存并发布')}
        </Button>
      </div>

        </>
      )}
      library={(
        <LibraryPanel
          groups={libraryGroups}
          onAdd={(_groupKey, item) => {
            const def = MODULE_DEF_MAP.get(item.key);
            if (def) addModule(def);
          }}
          onDragStartItem={(_groupKey, item, e) => {
            e.dataTransfer.setData('text/pc-decorate-lib', item.key);
            e.dataTransfer.effectAllowed = 'copy';
          }}
        />
      )}
      settings={(
        <>
          {isLoginPage && (
            <SettingsCard>
              <PcLoginSettingsPanel
                data={activeModule?.type === 'pcLogin' ? activeModule.module : (moduleList.find((m) => m.type === 'pcLogin')?.module || PC_LOGIN_DEFAULTS)}
                onChange={(patch) => {
                  const index = moduleList.findIndex((m) => m.type === 'pcLogin');
                  if (index < 0) return;
                  setModuleList((prev) => prev.map((m, i) => (i === index ? { ...m, module: { ...m.module, ...patch } } : m)));
                  markDirty();
                }}
              />
            </SettingsCard>
          )}
          {!isLoginPage && panelTab === 'page' && (
            <SettingsCard>
              <PageSettingsPanel
                pageModule={pageModule}
                legacyCount={moduleList.filter((m) => LEGACY_TYPES.has(m.type)).length}
                newCount={moduleList.filter((m) => !LEGACY_TYPES.has(m.type)).length}
                onChange={(next) => { setPageModule(next); markDirty(); }}
                onSwitchVersion={(target) => switchHomeVersion(target)}
              />
            </SettingsCard>
          )}
          {!isLoginPage && panelTab === 'pcHeader' && (
            <SettingsCard>
              <PcHeaderPanel header={pageModule.pcHeader || {}} onChange={(next) => { setPageModule({ ...pageModule, pcHeader: next }); markDirty(); }} />
            </SettingsCard>
          )}
          {!isLoginPage && panelTab === 'module' && (
            activeModule && activeDef ? (
              <ModuleSettingsPanel
                def={activeDef}
                moduleData={activeModule.module}
                onChange={(next) => updateActiveModule(() => next)}
              />
            ) : (
              <SettingsCard>
                <Text type="tertiary">{t('请在画布中选择一个模块，或切换到「页面设置 / PC头部装修」')}</Text>
              </SettingsCard>
            )
          )}
        </>
      )}
    >
        {/* 中间画布：1600px 整页无缝渲染（对齐原版 CanvasViewport，与 C 端一致） */}
        <div style={{ height: '100%', display: 'flex', flexDirection: 'column', minWidth: 0, background: '#eef1f6' }}>
          <div ref={canvasViewportRef} className="pc-decorate-canvas-viewport" style={{ flex: 1, overflowY: 'auto', overflowX: 'hidden' }}>
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'flex-start', padding: '24px 0 60px', position: 'relative' }}>
              {/* 1600px 整页窗口（等比缩放到容器宽度，背景取页面设置）。
                  外层宽 = 1600 * scale：flex 用缩放后的实际宽度居中，
                  避免宽屏时 zoom 后的画布向左溢出到模块库下方。 */}
              <div style={{ width: 1600 * canvasScale, flexShrink: 0, position: 'relative' }}>
              <div
                className="pc-decorate-window"
                style={{
                  zoom: canvasScale,
                  flexShrink: 0,
                  background: pageModule.backgroundColor || '#fff',
                  ...themeVars,
                  ...(pageModule.backgroundImage?.picUrl
                    ? {
                        backgroundImage: `url(${pageModule.backgroundImage.picUrl})`,
                        backgroundRepeat: (['no-repeat', 'repeat', 'repeat-y', 'repeat-x'][Number(pageModule.backgroundRepeat) || 1] || 'no-repeat') as any,
                        backgroundSize: (['100% auto', 'auto 100%', '100% 100%'][Number(pageModule.backgroundSize) || 1] || '100% auto') as any,
                      }
                    : {}),
                }}
                onDragOver={(e) => e.preventDefault()}
                onDrop={(e) => {
                  e.preventDefault();
                  const libType = e.dataTransfer.getData('text/pc-decorate-lib');
                  if (libType) {
                    const def = MODULE_DEF_MAP.get(libType);
                    if (def) addModuleAt(def, moduleList.length);
                  }
                }}
              >
                {!isLoginPage && (
                  <PcHeaderPreviewStrip
                    header={pageModule.pcHeader || {}}
                    regionText={regionText}
                    active={panelTab === 'pcHeader'}
                    onOpen={() => { setPanelTab('pcHeader'); setActiveIndex(-1); }}
                  />
                )}
                {moduleList.length === 0 && (
                  <div style={{ padding: 100, textAlign: 'center', color: '#c0c4cc' }}>{t('暂无模块，请从左侧模块库拖入或点击添加')}</div>
                )}
                {moduleList.map((item, index) => {
                  const isActive = activeIndex === index;
                  const isLegacy = LEGACY_TYPES.has(item.type);
                  return (
                    <div
                      key={item.moduleIndex}
                      data-module-index={index}
                      ref={(el) => {
                        if (el) moduleElRefs.current.set(item.moduleIndex, el);
                        else moduleElRefs.current.delete(item.moduleIndex);
                      }}
                      className={`pc-decorate-module${isActive ? ' pc-decorate-module--active' : ''}${item.isShow ? '' : ' pc-decorate-module--hidden'}`}
                      style={{ position: 'relative' }}
                      onDragOver={(e) => e.preventDefault()}
                      onDrop={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        const libType = e.dataTransfer.getData('text/pc-decorate-lib');
                        if (libType) {
                          const def = MODULE_DEF_MAP.get(libType);
                          if (def) addModuleAt(def, index);
                          return;
                        }
                        const from = Number(e.dataTransfer.getData('text/pc-decorate-index'));
                        if (!Number.isNaN(from)) dropModule(from, index);
                      }}
                      onClick={(e) => {
                        e.stopPropagation();
                        setActiveIndex(index);
                        setPanelTab('module');
                      }}
                    >
                      {isActive && <span className="pc-decorate-module__tag">{t(item.label)}{isLegacy ? '（' + t('旧版') + '）' : ''}</span>}
                      <ModuleErrorBoundary key={item.moduleIndex} label={item.type}>
                        <ModulePreview type={item.type} data={item.module} />
                      </ModuleErrorBoundary>
                    </div>
                  );
                })}
              </div>
              </div>
              {/* 右侧悬浮操作条：图标按钮，垂直居中，与设置栏保留间距 */}
              <div
                style={{
                  width: 56,
                  flexShrink: 0,
                  position: 'relative',
                  alignSelf: 'stretch',
                }}
              >
                {activeIndex >= 0 && activeIndex < moduleList.length && (
                  <div
                    className="pc-decorate-rail-operate"
                    style={{
                      top: ((railTops[moduleList[activeIndex]?.moduleIndex] ?? 0)
                        + (railHeights[moduleList[activeIndex]?.moduleIndex] ?? 0) / 2) * canvasScale,
                      transform: 'translateY(-50%)',
                    }}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <Tooltip content={t('上移')} position="left">
                      <Button size="small" theme="borderless" icon={<IconChevronUp />} disabled={activeIndex === 0} onClick={() => moveModule(activeIndex, -1)} />
                    </Tooltip>
                    <Tooltip content={t('下移')} position="left">
                      <Button size="small" theme="borderless" icon={<IconChevronDown />} disabled={activeIndex === moduleList.length - 1} onClick={() => moveModule(activeIndex, 1)} />
                    </Tooltip>
                    <Tooltip content={moduleList[activeIndex]?.isShow ? t('隐藏') : t('显示')} position="left">
                      <Button
                        size="small"
                        theme="borderless"
                        icon={moduleList[activeIndex]?.isShow ? <IconEyeOpened /> : <IconEyeClosed />}
                        onClick={() => toggleShow(activeIndex)}
                      />
                    </Tooltip>
                    <Tooltip content={t('复制')} position="left">
                      <Button size="small" theme="borderless" icon={<IconCopy />} onClick={() => copyModule(activeIndex)} />
                    </Tooltip>
                    <Tooltip content={t('删除')} position="left">
                      <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} onClick={() => removeModule(activeIndex)} />
                    </Tooltip>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>

    </EditorShell>
  );
}

/* ===================== 画布预览（轻量真实结构渲染） ===================== */

/** 模块标题预览（对齐原版 _shared/ModuleTitle.vue：样式一/二/三 装饰、对齐、背景色、圆角、更多/倒计时） */
function PreviewTitle({ title }: { title?: Record<string, any> }) {
  const { t } = useI18n();
  if (!title || Number(title.showTitle) !== 1) return null;
  const align = Number(title.titleAlign) || 1;
  const style = Number(title.titleStyle) || 1;
  const moreColor = title.moreColor || '#000';
  const containerStyle: Record<string, any> = {
    display: 'flex',
    alignItems: 'center',
    lineHeight: '30px',
    padding: '5px 10px 3px',
    position: 'relative',
    justifyContent: align === 2 ? 'center' : align === 3 ? 'flex-end' : 'space-between',
  };
  if (align === 3) containerStyle.flexDirection = 'row-reverse';
  if (title.titleBackground) containerStyle.backgroundColor = title.titleBackground;
  if (Number(title.titleRadius)) containerStyle.borderRadius = `${Number(title.titleRadius)}px ${Number(title.titleRadius)}px 0 0`;
  return (
    <div style={containerStyle}>
      <div style={{ display: 'flex', flexWrap: 'nowrap', alignItems: 'center', flexDirection: align === 2 ? 'column' : align === 3 ? 'row-reverse' : undefined, textAlign: align === 2 ? 'center' : undefined }}>
        {title.titleBackgroundPic?.picUrl ? (
          <img src={title.titleBackgroundPic.picUrl} alt="" style={{ width: 30, height: 30, marginRight: align === 2 || align === 3 ? 0 : 5, objectFit: 'contain', flexShrink: 0 }} />
        ) : null}
        <div style={{ position: 'relative', fontWeight: 700, fontSize: 16, color: title.titleColor || '#2a3145', paddingLeft: style === 2 ? 10 : undefined }}>
          {style === 2 && (
            <span style={{ position: 'absolute', left: 0, top: '50%', transform: 'translateY(-50%)', width: 3, height: 16, borderRadius: 2, background: 'linear-gradient(180deg, #155bd4 0%, #64b5f6 100%)' }} />
          )}
          {title.titleText || ''}
          {style === 3 && (
            <span style={{ position: 'absolute', left: 0, bottom: -2, width: '100%', height: 2, borderRadius: 1, background: 'linear-gradient(90deg, #155bd4 0%, transparent 100%)' }} />
          )}
        </div>
        {title.descText ? (
          <div style={{ fontSize: 12, color: title.descColor || '#aaaaaa', marginLeft: align === 2 ? 0 : 5, marginTop: align === 2 ? 2 : 0, position: 'relative', top: align === 2 ? undefined : 1 }}>
            {title.descText}
          </div>
        ) : null}
      </div>
      {Number(title.showMore) === 1 && Number(title.moreType ?? 1) === 1 && (
        <span style={{ display: 'flex', alignItems: 'center', fontSize: 12, color: moreColor, ...(align === 2 ? { position: 'absolute' as const, right: 10 } : {}) }}>
          {getPath(title, 'moreLink.title') || getPath(title, 'moreLink.name') || t('更多')} ›
        </span>
      )}
      {Number(title.moreType) === 2 && (
        <span style={{ fontSize: 12, color: moreColor, ...(align === 2 ? { position: 'absolute' as const, right: 10 } : {}) }}>{title.countdownLabel || t('倒计时')} 00:00:00</span>
      )}
    </div>
  );
}

/** 模块 frame → 内联样式（背景/内边距/圆角，与 C 端渲染一致） */
function frameStyle(m: Record<string, any>): Record<string, any> {
  const f = m?.frame;
  if (!f || typeof f !== 'object') return {};
  const style: Record<string, any> = {};
  if (f.backgroundColor) style.background = f.backgroundColor;
  if (Number(f.boxRadius)) style.borderRadius = Number(f.boxRadius);
  const top = Number(f.boxPaddingTop) || 0;
  const bottom = Number(f.boxPaddingBottom) || 0;
  const side = Number(f.boxPadding) || 0;
  const inner = Number(f.leftAndRightPadding) || 0;
  style.padding = `${top}px ${side + inner}px ${bottom}px ${side + inner}px`;
  return style;
}

interface PreviewProduct {
  productId?: number;
  productName?: string;
  picUrl?: string;
  picThumb?: string;
  productPrice?: number | string;
  marketPrice?: number | string;
  minGroupPrice?: number | string;
  minGroupNum?: number | string;
  categoryId?: number;
  categoryName?: string;
  isHot?: number;
  isShow?: number;
  picUrl1?: string;
}

/** 按选品配置拉取真实商品（与原版 Preview 取数口径一致） */
function usePreviewProducts(products?: Record<string, any>) {
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  const key = products ? JSON.stringify(products) : '';
  useEffect(() => {
    if (!products) {
      setRows([]);
      return;
    }
    const type = Number(products.productSelectType) || 1;
    let params: Record<string, unknown> | null = null;
    if (type === 1) {
      const ids = (products.productIds || []) as number[];
      if (!ids.length) {
        setRows([]);
        return;
      }
      params = { ids: ids.join(','), size: ids.length, page: 1, productStatus: 1, checkStatus: 1 };
    } else if (type === 2) {
      params = { categoryId: products.productCategoryId || 1, page: 1, size: products.productNumber || 10, productStatus: 1, checkStatus: 1 };
    } else {
      params = { introType: products.productTag || '', page: 1, size: products.productNumber || 10, productStatus: 1, checkStatus: 1 };
    }
    let alive = true;
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/product/product/list', params)
      .then((res) => {
        if (alive) setRows(res.records || res.list || []);
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, [key]);
  return rows;
}


/** 拼团活动商品（promotion/groupon/item/list） */
function useGrouponItems(promotionId?: number) {
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  useEffect(() => {
    if (!promotionId) {
      setRows([]);
      return;
    }
    let alive = true;
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/promotion/groupon/item/list', { grouponPromotionId: promotionId, page: 1, size: 20 })
      .then((res) => {
        if (alive) setRows(res.records || res.list || []);
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, [promotionId]);
  return rows;
}

/** 热门分类（product/category/list 过滤 isHot+isShow） */
function useHotCategories() {
  const [rows, setRows] = useState<PreviewProduct[]>([]);
  useEffect(() => {
    let alive = true;
    apiGet<{ records?: PreviewProduct[]; list?: PreviewProduct[] }>('/product/category/list', { page: 1, size: 100, parentId: -1 })
      .then((res) => {
        if (!alive) return;
        const all = res.records || res.list || [];
        setRows(all.filter((c) => Number(c.isHot) === 1 && Number(c.isShow) === 1));
      })
      .catch(() => {
        if (alive) setRows([]);
      });
    return () => {
      alive = false;
    };
  }, []);
  return rows;
}

/** C 端风格商品卡（真实数据） */
function ProductCards({
  rows,
  cols,
  emptyHint,
  priceField = 'productPrice',
  groupMode = false,
}: {
  rows: PreviewProduct[];
  cols: number;
  emptyHint?: string;
  priceField?: string;
  groupMode?: boolean;
}) {
  const { t } = useI18n();
  if (!rows.length) {
    return <div style={{ padding: 30, textAlign: 'center', color: '#c0c4cc', fontSize: 13 }}>{emptyHint || t('暂无商品数据')}</div>;
  }
  return (
    <div style={{ display: 'grid', gridTemplateColumns: `repeat(${cols}, 1fr)`, gap: 10, padding: 10 }}>
      {rows.slice(0, 30).map((p, i) => (
        <div key={p.productId ?? i} style={{ background: '#fff', borderRadius: 6, overflow: 'hidden', border: '1px solid #f2f3f5' }}>
          <div style={{ aspectRatio: '1 / 1', background: '#f7f8fa', overflow: 'hidden' }}>
            {(p.picThumb || p.picUrl) && <img src={p.picThumb || p.picUrl} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />}
          </div>
          <div style={{ padding: 6 }}>
            {p.productName && (
              <div style={{ fontSize: 12, color: '#333', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: Number(cols) >= 4 ? 'nowrap' : 'normal', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' as any }}>
                {p.productName}
              </div>
            )}
            <div style={{ marginTop: 4, display: 'flex', alignItems: 'baseline', gap: 4 }}>
              <span style={{ color: '#e1251b', fontWeight: 700, fontSize: 14 }}>
                ¥{Number((p as any)[priceField] ?? p.productPrice ?? 0).toFixed(2)}
              </span>
              {groupMode && p.minGroupNum ? <span style={{ fontSize: 11, color: '#999' }}>{t('{count}人团', { count: p.minGroupNum })}</span> : null}
              {p.marketPrice && Number(p.marketPrice) > 0 ? (
                <span style={{ fontSize: 11, color: '#999', textDecoration: 'line-through' }}>¥{Number(p.marketPrice).toFixed(2)}</span>
              ) : null}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

function PcLoginSettingsPanel({
  data,
  onChange,
}: {
  data: Record<string, any>;
  onChange: (patch: Record<string, any>) => void;
}) {
  const { t } = useI18n();
  const d = { ...PC_LOGIN_DEFAULTS, ...(data || {}) };
  const sliderValue = (value: unknown, fallback: number) => (Array.isArray(value) ? Number(value[0]) : Number(value) || fallback);
  return (
    <div className="pc-login-settings">
      <div className="pc-login-settings__title">
        <h3>{t('登录页')}</h3>
        <p>{t('选择 PC 登录页模式并配置背景，保存发布后前台按此渲染')}</p>
      </div>

      <div className="pc-login-field">
        <label>{t('登录模式')}</label>
        <RadioGroup
          type="button"
          value={Number(d.loginMode) || 2}
          onChange={(e) => onChange({ loginMode: Number(e.target.value) })}
        >
          <Radio value={1}>{t('经典版')}</Radio>
          <Radio value={2}>{t('简约版')}</Radio>
        </RadioGroup>
      </div>

      <div className="pc-login-field">
        <label>{t('头部高度')}</label>
        <SliderWithInput
          min={50}
          max={120}
          value={sliderValue(d.headerHeight, 80)}
          onChange={(v) => onChange({ headerHeight: v })}
        />
      </div>

      <div className="pc-login-field">
        <label>{t('logo宽度')}</label>
        <SliderWithInput
          min={10}
          max={160}
          value={sliderValue(d.logoWidth, 100)}
          onChange={(v) => onChange({ logoWidth: v })}
        />
      </div>

      <div className="pc-login-field">
        <label>{t('logo高度')}</label>
        <SliderWithInput
          min={10}
          max={60}
          value={sliderValue(d.logoHeight, 25)}
          onChange={(v) => onChange({ logoHeight: v })}
        />
      </div>

      <div className="pc-login-field pc-login-field--switch">
        <label>{t('头部阴影')}</label>
        <Switch checked={Boolean(d.headerShadow)} onChange={(checked) => onChange({ headerShadow: checked })} />
      </div>

      <div className="pc-login-field pc-login-field--switch">
        <label>{t('头部欢迎语')}</label>
        <Switch checked={Boolean(d.showWelcome)} onChange={(checked) => onChange({ showWelcome: checked })} />
      </div>

      <div className="pc-login-settings__section">{t('背景设置')}</div>

      <div className="pc-login-field pc-login-field--block">
        <label>{t('背景图')}</label>
        <ImageUploader
          single
          value={d.backgroundPic?.picUrl ? [d.backgroundPic.picUrl] : []}
          onChange={(next) => onChange({ backgroundPic: { ...(d.backgroundPic || {}), picUrl: next[0] || '' } })}
        />
        <p className="pc-login-field__help">{t('建议高度不超过 580px、宽度不超过 1700px，可配合下方背景色使用')}</p>
      </div>

      <div className="pc-login-field">
        <label>{t('背景色')}</label>
        <SharedColorField
          value={d.backgroundColor || ''}
          onChange={(v) => onChange({ backgroundColor: v })}
          resetValue=""
        />
      </div>

      <div className="pc-login-field">
        <label>{t('登录区高度')}</label>
        <SliderWithInput
          min={620}
          max={800}
          value={Math.max(620, sliderValue(d.warpHeight, 620))}
          onChange={(v) => onChange({ warpHeight: v })}
        />
      </div>
    </div>
  );
}


/** 单模块错误边界：一个模块预览抛错时只降级该模块，不再整页白屏
 * （无边界时 React 会卸载整个应用，2026-11-13 优惠券标题 #31 白屏即此机制）。
 * 错误仍打到 console，不吞。 */
class ModuleErrorBoundary extends Component<{ label?: string; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError(): { failed: boolean } {
    return { failed: true };
  }

  componentDidCatch(error: Error, _info: ErrorInfo): void {
    console.error('[pc-decorate] module preview failed:', this.props.label || '', error);
  }

  render(): ReactNode {
    if (this.state.failed) {
      return (
        <div style={{ padding: '24px 10px', textAlign: 'center', background: '#fff5f5', color: '#c92a2a', fontSize: 13 }}>
          {tl('该模块预览失败，已跳过渲染（不影响其他模块）')}
        </div>
      );
    }
    return this.props.children;
  }
}

function ModulePreview({ type, data }: { type: string; data: Record<string, any> }) {
  const { t } = useI18n();
  const m = data || {};
  const Preview = PC_MODULE_PREVIEWS[type];
  if (Preview) return <Preview data={m} />;
  return <span>{MODULE_DEF_MAP.get(type)?.summary ? MODULE_DEF_MAP.get(type)!.summary!(data) : type}</span>;
}

/* ---- 真实数据预览子组件（hooks 按实例隔离） ---- */

/** 热图绘制预览（对齐原版 dec-image-hotarea：图片纵向堆叠，热区按 500px 基准随容器宽度等比缩放） */
function HotareaPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const pics = (data.picList || []) as Array<PicItem & { hotarea?: Array<Record<string, any>> }>;
  const f = (data.frame || {}) as Record<string, any>;
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const [wrapWidth, setWrapWidth] = useState(0);
  useEffect(() => {
    const el = wrapRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setWrapWidth(Number((entry.target as HTMLElement).offsetWidth.toFixed(2)));
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [pics.length]);
  const scale = (v?: number) => ((wrapWidth || 0) / 500) * (Number(v) || 0);
  return (
    <div style={{ padding: `${Number(f.boxPaddingTop ?? 5)}px ${Number(f.boxPadding ?? 10)}px ${Number(f.boxPaddingBottom ?? 5)}px` }}>
      <div style={{ backgroundColor: f.backgroundColor || '#ffffff', borderRadius: Number(f.boxRadius) || 0, overflow: 'hidden', padding: Number(f.leftAndRightPadding) || 0 }}>
        <PreviewTitle title={data.title} />
        {pics.length === 0 ? (
          <div style={{ paddingTop: 30, paddingBottom: 50, textAlign: 'center', backgroundColor: '#f2f4f6', color: '#666', fontSize: 14 }}>
            <div style={{ width: 100, height: 100, margin: '0 auto 10px', background: '#e5e9f0', borderRadius: 8 }} />
            {t('点击添加热图绘制')}
          </div>
        ) : (
          <div style={{ overflow: 'hidden' }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: Number(data.imgPadding) || 0 }}>
              {pics.map((pic, pi) => (
                <div key={pi} style={{ position: 'relative', overflow: 'hidden' }} ref={pi === 0 ? wrapRef : undefined}>
                  {pic.picUrl ? (
                    <img src={pic.picUrl} alt={pic.picTitle || ''} style={{ display: 'block', width: '100%', height: 'auto', objectFit: 'cover' }} />
                  ) : (
                    <div style={{ width: '100%', paddingTop: '40%', background: '#f2f4f6' }} />
                  )}
                  {(pic.hotarea || []).map((h, hi) => (
                    <div
                      key={hi}
                      style={{
                        position: 'absolute',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        background: 'rgba(21, 91, 212, 0.6)',
                        color: '#fff',
                        boxSizing: 'border-box',
                        border: '1px solid transparent',
                        width: scale(h.width),
                        height: scale(h.height),
                        top: scale(h.top),
                        left: scale(h.left),
                      }}
                    >
                      <span style={{ fontSize: 12, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', padding: '0 4px' }}>{h.link?.name || ''}</span>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** 商品模块预览（对齐原版 dec-pc-product：4/5 列栅格 gap 12、商品边距包裹层、卡片描边、名称/描述/价格开关） */
function ProductPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const rows = usePreviewProducts(data.products);
  const f = (data.frame || {}) as Record<string, any>;
  const cols = Number(data.style) === 2 ? 5 : 4;
  const hasData = Number(data.products?.productSelectType || 0) === 1
    ? ((data.products?.productIds || []) as number[]).length > 0
    : Number(data.products?.productSelectType || 0) > 0;
  return (
    <div style={{ padding: `${Number(f.boxPaddingTop ?? 5)}px ${Number(f.boxPadding ?? 10)}px ${Number(f.boxPaddingBottom ?? 5)}px`, backgroundColor: f.backgroundColor || data.backgroundColor || '#ffffff', borderRadius: Number(f.boxRadius) || 0 }}>
      <div style={{ padding: `0 ${Number(f.leftAndRightPadding) || 0}px` }}>
        <PreviewTitle title={data.title} />
        {!hasData || rows.length === 0 ? (
          <div style={{ paddingTop: 30, paddingBottom: 50, textAlign: 'center', backgroundColor: '#f2f4f6', color: '#666', fontSize: 14 }}>
            <div style={{ width: 100, height: 100, margin: '0 auto 10px', background: '#e5e9f0', borderRadius: 8 }} />
            {t('点击添加商品')}
          </div>
        ) : (
          <div style={{ padding: Number(data.goodsPadding ?? 5) }}>
            <div style={{ display: 'grid', gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`, gap: 12 }}>
              {rows.slice(0, 30).map((p, i) => (
                <div key={p.productId ?? i} style={{ minWidth: 0 }}>
                  <div style={{ display: 'block', border: '1px solid rgba(50, 50, 51, 0.08)', borderRadius: 6, overflow: 'hidden', background: '#fff' }}>
                    <div style={{ width: '100%', aspectRatio: '1 / 1', background: '#f5f5f5', overflow: 'hidden' }}>
                      {(p.picThumb || p.picUrl) && <img src={p.picThumb || p.picUrl} alt={p.productName || ''} style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />}
                    </div>
                    <div style={{ padding: '10px 10px 12px' }}>
                      {Number(data.showName) !== 0 && (
                        <div style={{ fontSize: 13, lineHeight: '18px', color: '#333', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.productName}</div>
                      )}
                      {Number(data.showBrief) !== 0 && (
                        <div style={{ marginTop: 6, fontSize: 12, lineHeight: '16px', color: '#999', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{(p as any).brief || t('商品描述')}</div>
                      )}
                      {Number(data.showPrice) !== 0 && (
                        <div style={{ marginTop: 8, fontSize: 14, fontWeight: 600, color: '#e60012' }}>¥{Number(p.productPrice ?? 0).toFixed(2)}</div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** 推荐商品预览（对齐原版 dec-pc-recommend-product：大标题+下划线、columns/itemGap 栅格、白卡 1:1 contain 图、加购圆钮） */
function RecommendPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const rows = usePreviewProducts(data.products);
  const f = (data.frame || {}) as Record<string, any>;
  const columns = Number(f.columns) || 5;
  const itemGap = Number(f.itemGap ?? 16);
  return (
    <div style={{ padding: `${Number(f.boxPaddingTop ?? 0)}px ${Number(f.boxPadding ?? 10)}px 0`, backgroundColor: f.backgroundColor || 'transparent', borderRadius: Number(f.boxRadius) || 0 }}>
      {!rows.length ? (
        <div style={{ minHeight: 120, borderRadius: 6, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', padding: '30px 0 50px', textAlign: 'center', backgroundColor: '#f2f4f6', color: '#666', fontSize: 14 }}>
          <div style={{ width: 100, height: 100, margin: '0 auto 10px', background: '#e5e9f0', borderRadius: 8 }} />
          {t('点击添加商品')}
        </div>
      ) : (
        <>
          {data.titleName ? (
            <h3 style={{ position: 'relative', width: 'fit-content', margin: 0, whiteSpace: 'nowrap', fontSize: 24, fontWeight: 700, color: '#333333', textAlign: 'left', paddingBottom: 4, overflow: 'hidden', display: 'flex', alignItems: 'flex-start' }}>
              {data.titleName}
              <span style={{ position: 'absolute', width: 36, left: '50%', bottom: 0, height: 3, borderRadius: 1, backgroundColor: '#1a1a1a', transform: 'translateX(-50%)' }} />
            </h3>
          ) : null}
          <div style={{ marginTop: data.titleName ? 32 : 0, overflow: 'hidden', display: 'grid', gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gap: itemGap, paddingBottom: Number(f.boxPaddingBottom ?? 50) }}>
            {rows.slice(0, 30).map((p, i) => (
              <div key={p.productId ?? i} style={{ boxSizing: 'border-box', width: '100%', borderRadius: 4, paddingBottom: 12, background: '#ffffff', overflow: 'hidden', border: '1px solid transparent' }}>
                <div style={{ width: '100%', aspectRatio: '1 / 1', background: '#ffffff', overflow: 'hidden' }}>
                  {(p.picThumb || p.picUrl) && <img src={p.picThumb || p.picUrl} alt={p.productName || ''} style={{ width: '100%', height: '100%', display: 'block', objectFit: 'contain' }} />}
                </div>
                <div style={{ padding: '8px 12px 0', marginTop: 8 }}>
                  <div style={{ fontSize: 16, fontWeight: 400, lineHeight: '22px', wordBreak: 'break-word', textAlign: 'left', overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' as any, color: '#19191b', minHeight: 44 }}>{p.productName}</div>
                  <div style={{ minHeight: 18, marginTop: 10 }}>
                    <div style={{ fontSize: 14, fontWeight: 400, lineHeight: '18px', color: '#888b94', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{t('已售')} {(p as any).virtualSales ?? 0}</div>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 6, minHeight: 32 }}>
                    <div style={{ display: 'flex', alignItems: 'baseline', color: '#be081d', fontWeight: 600 }}>
                      <span style={{ fontSize: 16, fontWeight: 400 }}>¥</span>
                      <span style={{ fontSize: 28, fontWeight: 600, lineHeight: '28px' }}>{Number(p.productPrice ?? 0).toFixed(2).split('.')[0]}</span>
                      <span style={{ fontSize: 16, fontWeight: 400 }}>.{Number(p.productPrice ?? 0).toFixed(2).split('.')[1] || '00'}</span>
                    </div>
                    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', width: 32, height: 32, borderRadius: '50%', background: 'rgba(255, 63, 79, 0.12)', color: '#ff3f4f', flexShrink: 0 }}>
                      <IconPlus />
                    </div>
                  </div>
                  <div style={{ fontSize: 14, lineHeight: '18px', color: '#269636', fontWeight: 600, marginTop: 6, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {(p as any).shopTitle || t('官方自营')}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}



/** 分类导航预览（对齐原版 dec-pc-category-nav：外层 frame 留白、内层背景/圆角/固定高、一屏 displayNum 个、圆形箭头） */
function CategoryNavPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const rows = useHotCategories();
  const f = (data.frame || {}) as Record<string, any>;
  const displayNum = Math.min(10, Math.max(1, Number(data.displayNum) || 6));
  const display = rows.slice(0, displayNum);
  return (
    <div
      style={{
        boxSizing: 'border-box',
        backgroundColor: f.backgroundColor || 'transparent',
        borderRadius: Number(f.boxRadius ?? 8),
        overflow: Number(f.boxRadius) ? 'hidden' : undefined,
        padding: `${Number(f.boxPaddingTop) || 0}px ${Number(f.boxPadding) || 0}px ${Number(f.boxPaddingBottom) || 0}px`,
      }}
    >
      <div style={{ backgroundColor: data.backgroundColor || '#f7f7f7', borderRadius: 8, height: Number(f.boxHeight ?? 328), display: 'flex', flexDirection: 'column', overflow: 'hidden', width: '100%', boxSizing: 'border-box' }}>
        {Number(data.showTitle) === 1 && (
          <div style={{ padding: '20px 24px', fontSize: 24, fontWeight: 700, color: '#1a1a1a' }}>{data.titleName || t('热门分类')}</div>
        )}
        <div style={{ position: 'relative', flex: 1, width: '100%', minHeight: 0 }}>
          {display.length === 0 ? (
            <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', padding: '0 24px', color: '#999', fontSize: 14 }}>
              {t('暂无热门分类（请在分类管理中勾选"热门"）')}
            </div>
          ) : (
            <div style={{ display: 'flex', height: '100%', overflow: 'hidden' }}>
              {display.map((c, i) => (
                <div key={c.categoryId ?? i} style={{ flex: 1, minWidth: 0, boxSizing: 'border-box' }}>
                  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center' }}>
                    <div style={{ width: 'min(180px, 100%)', aspectRatio: '1', background: '#ecf5ff', borderRadius: 4, overflow: 'hidden' }}>
                      {((c as any).picUrl || (c as any).pic)
                        ? <img src={(c as any).picUrl || (c as any).pic} alt={c.categoryName || ''} style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
                        : <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E" alt="" style={{ width: '100%', height: '100%', display: 'block' }} />}
                    </div>
                    <div style={{ maxWidth: 180, width: '100%', marginTop: 12, marginBottom: 8, textAlign: 'center', color: '#1a1a1a', fontSize: 16, lineHeight: '20px', overflow: 'hidden', display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical' as any }}>
                      {c.categoryName}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
          {display.length > 0 && (
            <>
              <div className="pc-decorate-swiper-arrow pc-decorate-swiper-arrow--prev"><span /></div>
              <div className="pc-decorate-swiper-arrow pc-decorate-swiper-arrow--next"><span /></div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

/** 快捷导航预览（对齐原版 dec-pc-home-quick-nav：frame 留白/背景 + nav_container fs_mod 图左文右横排） */
function QuickNavPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const f = (data.frame || {}) as Record<string, any>;
  const s = (data.style || {}) as Record<string, any>;
  const items = (data.items || []) as PicItem[];
  const align = Number(s.containerAlign ?? 1);
  const justify = align === 2 ? 'center' : align === 3 ? 'flex-end' : 'flex-start';
  const iconSize = Number(s.iconSize ?? 16);
  return (
    <div
      style={{
        boxSizing: 'border-box',
        padding: `${Number(f.boxPaddingTop) || 0}px ${Number(f.boxPadding) || 0}px ${Number(f.boxPaddingBottom) || 0}px`,
        backgroundColor: f.backgroundColor || 'transparent',
        borderRadius: Number(f.boxRadius) || 0,
      }}
    >
      <div
        style={{
          display: 'flex',
          flexWrap: Number(s.singleLine) === 1 ? 'nowrap' : 'wrap',
          alignItems: 'center',
          justifyContent: justify,
          gap: Number(s.itemGap ?? 20),
          overflow: 'hidden',
          lineHeight: `${Number(s.lineHeight ?? 20)}px`,
          fontWeight: Number(s.fontWeight ?? 400),
        }}
      >
        {items.length === 0 ? (
          <div style={{ padding: '16px 0', textAlign: 'center', color: '#999', fontSize: 14, width: '100%' }}>{t('请在设置中添加导航项')}</div>
        ) : (
          items.map((item, i) => (
            <a
              key={item.picId || i}
              className="pc-quick-nav-item"
              style={
                {
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: Number(s.iconGap ?? 4),
                  fontSize: Number(s.fontSize ?? 14),
                  fontWeight: Number(s.fontWeight ?? 400),
                  lineHeight: `${Number(s.lineHeight ?? 20)}px`,
                  color: s.textColor || '#333333',
                  textDecoration: 'none',
                  whiteSpace: 'nowrap',
                  cursor: 'pointer',
                  '--quick-nav-hover-color': s.hoverColor || '#e1251b',
                } as Record<string, any>
              }
              title={item.picTitle || ''}
            >
              {(item.picUrl || item.picThumb) && (
                <img src={(item.picUrl || item.picThumb) as string} alt={item.picTitle || ''} style={{ flexShrink: 0, width: iconSize, height: iconSize, objectFit: 'contain' }} />
              )}
              <span style={{ flexShrink: 0 }}>{item.picTitle}</span>
            </a>
          ))
        )}
      </div>
    </div>
  );
}

/** 分类导航3预览（对齐原版 dec-pc-category-nav3：顶栏 24px 标题+更多、一屏 6 卡、图片背景色、火苗描述行） */
function CategoryNav3Preview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const f = (data.frame || {}) as Record<string, any>;
  const picList = (Array.isArray(data.picList) ? data.picList : []) as PicItem[];
  const bgColor = data.backgroundColor || f.backgroundColor || 'rgba(0, 0, 0, 0.02)';
  const moduleImageBg = data.imageBackgroundColor || '';
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        width: '100%',
        boxSizing: 'border-box',
        overflow: 'hidden',
        padding: `${Number(f.boxPaddingTop) || 0}px ${Number(f.boxPadding) || 0}px ${Number(f.boxPaddingBottom) || 0}px`,
        height: Number(f.boxHeight ?? 352),
        borderRadius: Number(f.boxRadius ?? 8),
        backgroundColor: bgColor,
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '20px 24px' }}>
        <div style={{ fontSize: 24, fontWeight: 700, color: '#1a1a1a' }}>{data.titleText || t('热门推荐')}</div>
        {Number(data.showMore) === 1 && (
          <span style={{ display: 'flex', alignItems: 'center', fontSize: 16, color: '#1a1a1a' }}>
            <span style={{ lineHeight: '20px' }}>{data.showMoreText || t('更多')}</span>
            <span style={{ marginLeft: 6, fontSize: 18, lineHeight: '18px' }}>&gt;</span>
          </span>
        )}
      </div>
      <div style={{ position: 'relative', flex: 1, height: '100%', width: '100%', minHeight: 0 }}>
        {picList.length === 0 ? (
          <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', minHeight: 120, borderRadius: 6, padding: '30px 0 50px', textAlign: 'center', backgroundColor: '#f2f4f6', color: '#666', fontSize: 14 }}>
            <div style={{ width: 100, height: 100, margin: '0 auto 10px', background: '#e5e9f0', borderRadius: 8 }} />
            {t('请在设置中添加内容')}
          </div>
        ) : (
          <div style={{ display: 'flex', width: '100%', height: '100%', overflowX: 'auto', overflowY: 'hidden', boxSizing: 'border-box', padding: '0 12px', scrollbarWidth: 'none' }}>
            {picList.map((item, i) => (
              <div key={String(item.picId || i)} style={{ boxSizing: 'border-box', flex: '0 0 calc(100% / 6)', minWidth: 0, height: '100%', padding: '0 14px' }}>
                <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
                  <div style={{ width: '100%', aspectRatio: '1', flex: '0 1 auto', minHeight: 0, backgroundColor: item.imageBackgroundColor || moduleImageBg || '#ecf5ff', borderRadius: 4, overflow: 'hidden' }}>
                    {(item.picUrl || item.picThumb)
                      ? <img src={(item.picUrl || item.picThumb) as string} alt={item.picTitle || ''} style={{ width: '100%', height: '100%', objectFit: 'contain', display: 'block' }} />
                      : <img src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E%3Crect fill='%23f0f0f0' width='180' height='180'/%3E%3C/svg%3E" alt="" style={{ width: '100%', height: '100%', display: 'block' }} />}
                  </div>
                  <div style={{ flex: 'none', width: '100%', height: 20, marginTop: 12, marginBottom: 8, textAlign: 'center', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis', color: '#1a1a1a', fontSize: 16, lineHeight: '20px' }}>
                    {item.picTitle}
                  </div>
                  {item.picDesc ? (
                    <div style={{ display: 'flex', flex: 'none', justifyContent: 'center', alignItems: 'center', width: '100%' }}>
                      <span style={{ color: '#888b94', fontSize: 16, lineHeight: '20px', overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>{item.picDesc}</span>
                    </div>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

/** 公告预览（对齐原版 dec-pc-notice：frame 留白/容器底 + itemHeight 圆角条 + 图标 16px） */
function NoticePreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const f = (data.frame || {}) as Record<string, any>;
  const displayText = String(data.text ?? '').trim();
  const icon = (data.icoPic || {}) as PicItem;
  return (
    <div
      style={{
        width: '100%',
        boxSizing: 'border-box',
        padding: `${Number(f.boxPaddingTop ?? 5)}px ${Number(f.boxPadding ?? 10)}px ${Number(f.boxPaddingBottom ?? 5)}px`,
        background: f.backgroundColor ?? '#ffffff',
        borderRadius: Number(f.boxRadius) || 0,
      }}
    >
      <div style={{ width: '100%' }}>
        {!displayText ? (
          <div style={{ width: '100%', height: 80, border: '1px dashed #dcdfe6', borderRadius: 8, display: 'flex', alignItems: 'center', justifyContent: 'center', background: '#fafafa' }}>
            <span style={{ color: '#909399', fontSize: 14 }}>{t('点击添加通知公告')}</span>
          </div>
        ) : (
          <div
            style={{
              width: '100%',
              height: Number(f.itemHeight) || 35,
              background: f.itemBackgroundColor ?? '#f3f7ff',
              borderRadius: Number(f.itemRadius) || 0,
              color: f.textColor ?? '#555555',
              display: 'flex',
              alignItems: 'center',
              boxSizing: 'border-box',
              padding: '0 10px',
            }}
          >
            {icon.picUrl ? <img src={icon.picUrl} alt="" style={{ width: 16, height: 16, flexShrink: 0, marginRight: 8, objectFit: 'contain', display: 'block' }} /> : null}
            <div style={{ flex: 1, fontSize: 14, lineHeight: 1.2, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{displayText}</div>
          </div>
        )}
      </div>
    </div>
  );
}

/** 文本标题预览（对齐原版 dec-pc-title-ad：风格一/二、居左/居中、背景渐变、查看更多） */
function TitleAdPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const titleData = (data.title || {}) as Record<string, any>;
  const f = (data.frame || {}) as Record<string, any>;
  const align = Number(titleData.titleAlign ?? 1);
  const style = Number(titleData.titleStyle ?? 1);
  const hasTitleText = String(titleData.titleText ?? '').trim().length > 0;
  const bg1 = String(titleData.titleBackground ?? '');
  const bg2 = String(titleData.titleBackground2 ?? '');
  const background = bg1 && bg2 ? `linear-gradient(to right, ${bg1}, ${bg2})` : bg1;
  const innerPadding = Number(f.leftAndRightPadding ?? 0);
  return (
    <div
      style={{
        background: f.backgroundColor ?? '#ffffff',
        borderRadius: Number(f.boxRadius) || 0,
        padding: `${Number(f.boxPaddingTop ?? 5)}px ${Number(f.boxPadding ?? 10)}px ${Number(f.boxPaddingBottom ?? 5)}px`,
        boxSizing: 'border-box',
      }}
    >
      <div style={{ paddingLeft: innerPadding, paddingRight: innerPadding }}>
        {!hasTitleText ? (
          <div style={{ padding: '30px 0 50px', textAlign: 'center', backgroundColor: '#f2f4f6', borderRadius: 6, color: '#888', fontSize: 14 }}>{t('点击添加标题')}</div>
        ) : (
          <div style={{ width: '100%' }}>
            <div
              style={{
                position: 'relative',
                display: 'flex',
                alignItems: 'center',
                justifyContent: align === 2 ? 'center' : 'space-between',
                gap: 12,
                padding: `10px ${Number(f.leftAndRightPadding ?? f.innerPadding ?? 0)}px`,
                boxSizing: 'border-box',
                overflow: 'hidden',
                borderRadius: Number(titleData.titleRadius) || 0,
                background: background || undefined,
              }}
            >
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8, flex: align === 2 ? 'none' : 1, overflow: 'hidden', alignItems: align === 2 ? 'center' : undefined, textAlign: align === 2 ? 'center' : undefined }}>
                <div style={{ lineHeight: '20px', fontSize: 16, fontWeight: style === 2 ? 600 : 'bold', color: titleData.titleColor || '#2a3145', wordBreak: 'break-all' }}>{titleData.titleText}</div>
                {titleData.descText ? (
                  <div
                    style={{
                      lineHeight: '18px',
                      paddingTop: style === 2 ? 0 : 8,
                      fontSize: style === 2 ? 16 : 12,
                      fontWeight: style === 2 ? 600 : undefined,
                      color: titleData.descColor || (style === 2 ? '#11bb55' : '#8c8c8c'),
                      wordBreak: 'break-all',
                    }}
                  >
                    {titleData.descText}
                  </div>
                ) : null}
              </div>
              {Number(titleData.showMore) === 1 && (
                <span
                  style={{
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    color: titleData.moreColor || '#000',
                    whiteSpace: 'nowrap',
                    ...(align === 2 ? { position: 'absolute' as const, right: 0 } : {}),
                  }}
                >
                  <span>{tl('查看更多')}</span>
                  <span style={{ fontSize: 12, lineHeight: '12px' }}>&gt;</span>
                </span>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/** 分割线预览（对齐原版 dec-divider：容器底色/圆角 + itemHeight 行高 + 实线/虚线/点状线） */
function DividerPreview({ data }: { data: Record<string, any> }) {
  const f = (data.frame || {}) as Record<string, any>;
  const lineType = Number(data.lineType ?? 2);
  const lineStyle = Number(data.lineStyle ?? 1);
  const lineNode =
    lineType === 2 ? (
      <div
        style={{
          width: '100%',
          ...(lineStyle === 2
            ? { height: 0, borderTop: '1px dashed #e5e5e5' }
            : lineStyle === 3
              ? { height: 0, borderTop: '1px dotted #e5e5e5' }
              : { height: 1, backgroundColor: '#e5e5e5' }),
        }}
      />
    ) : null;
  return (
    <div
      style={{
        boxSizing: 'border-box',
        paddingLeft: Number(f.boxPadding ?? 10),
        paddingRight: Number(f.boxPadding ?? 10),
        paddingTop: Number(f.boxPaddingTop ?? 5),
        paddingBottom: Number(f.boxPaddingBottom ?? 5),
      }}
    >
      <div style={{ backgroundColor: f.backgroundColor ?? '#ffffff', borderRadius: Number(f.boxRadius) || 0, overflow: 'hidden' }}>
        <div style={{ height: Number(f.itemHeight) || 10, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{lineNode}</div>
      </div>
    </div>
  );
}

/** 空白辅助预览（对齐原版 dec-pc-spacer：frame 留白 + blankHeight/背景/圆角） */
function SpacerPreview({ data }: { data: Record<string, any> }) {
  const f = (data.frame || {}) as Record<string, any>;
  return (
    <div
      style={{
        boxSizing: 'border-box',
        paddingLeft: Number(f.boxPadding) || 0,
        paddingRight: Number(f.boxPadding) || 0,
        paddingTop: Number(f.boxPaddingTop) || 0,
        paddingBottom: Number(f.boxPaddingBottom) || 0,
      }}
    >
      <div style={{ height: Number(data.blankHeight) || 30, background: data.backgroundColor || 'transparent', borderRadius: Number(f.boxRadius) || 0, boxSizing: 'border-box' }} />
    </div>
  );
}

/* ---- 秒杀/拼团共享预览原子（对齐原版 _shared/groupon/utils + utils/titleMore） ---- */

/** 1s 心跳时钟（倒计时实时走秒） */
function useNowTick(intervalMs = 1000) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/** "YYYY-MM-DD HH:mm:ss" → 时间戳（无效返回 0，口径同原版 dateStringToTimestamp） */
function dateStringToTimestamp(dateString?: string | null): number {
  if (!dateString) return 0;
  const parts = String(dateString).split(' ');
  const datePart = parts[0] || '';
  const timePart = parts[1] || '';
  const [year, month, day] = datePart.split('-');
  const [hour, minute, second] = timePart ? timePart.split(':') : ['0', '0', '0'];
  return new Date(Number(year), Number(month) - 1, Number(day), Number(hour), Number(minute), Number(second)).getTime();
}

function parseCountdownParts(remainingMs: number) {
  return {
    days: Math.floor(remainingMs / (1000 * 60 * 60 * 24)),
    hours: Math.floor((remainingMs % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60)),
    minutes: Math.floor((remainingMs % (1000 * 60 * 60)) / (1000 * 60)),
    seconds: Math.floor((remainingMs % (1000 * 60)) / 1000),
  };
}

const formatNumTwo = (n: number) => String(n).padStart(2, '0');

/** 三段式价格（int/dec），口径同原版 formatPriceParts */
function priceParts(price: string | number): { int: string; dec: string } {
  const num = Number(price);
  const text = Number.isFinite(num) ? num.toFixed(2) : String(price ?? '0.00');
  const [int, dec] = text.split('.');
  return { int: int || '0', dec: dec || '00' };
}

/** 右侧「更多」箭头（原版内联 SVG） */
function TitleMoreArrow() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" style={{ display: 'block' }}>
      <path d="M5.5 2.5l5 5.5-5 5.5" stroke="currentColor" strokeWidth="1.5" fill="none" />
    </svg>
  );
}

/** 标题右侧倒计时（深色数字方块，对齐前台 countdown-box） */
function TitleCountdownBox({ label, targetTs, now, color }: { label: string; targetTs: number; now: number; color: string }) {
  const { t } = useI18n();
  const remaining = targetTs ? Math.max(0, targetTs - now) : 0;
  const { days, hours, minutes, seconds } = parseCountdownParts(remaining);
  const boxStyle: Record<string, any> = {
    display: 'inline-block', padding: '2px 0', width: 32, textAlign: 'center',
    backgroundColor: '#1a1a1a', color: '#fff', borderRadius: 4, fontSize: 16,
  };
  return (
    <div style={{ display: 'flex', alignItems: 'center', fontSize: 18, color }}>
      <div style={{ display: 'flex', alignItems: 'center', color }}>
        <span style={{ fontSize: 18, fontWeight: 400, lineHeight: '24px' }}>{label}: </span>
        {days > 0 && (
          <>
            <span style={{ ...boxStyle, width: 'auto', minWidth: 32, padding: '2px 5px' }}>{days}</span>
            <span style={{ fontSize: 18, margin: '0 2px' }}>{t('天')}</span>
          </>
        )}
        <span style={boxStyle}>{formatNumTwo(hours)}</span>
        <span style={{ margin: '0 2px' }}>:</span>
        <span style={boxStyle}>{formatNumTwo(minutes)}</span>
        <span style={{ margin: '0 2px' }}>:</span>
        <span style={boxStyle}>{formatNumTwo(seconds)}</span>
      </div>
    </div>
  );
}

/**
 * 秒杀/拼团标题栏（对齐原版 Preview.vue groupon-top/dec-pc-home-seckill__top）：
 * titleStyle 3 竖线、标题图 30px、24px 粗标题、desc 12/16px、右侧倒计时方块或「更多」链接
 */
function SeckillGrouponTitleBar({
  title,
  fallbackText,
  fallbackLabel,
  countdownTargetTs,
  activityStartTs,
  now,
  innerPaddingPx,
}: {
  title?: Record<string, any>;
  fallbackText: string;
  fallbackLabel: string;
  countdownTargetTs: number;
  activityStartTs?: number;
  now: number;
  innerPaddingPx: number;
}) {
  const { t } = useI18n();
  if (!title || Number(title.showTitle) !== 1) return null;
  const showMore = Number(title.showMore) === 1;
  const moreTypeRaw = Number(title.moreType);
  const mode = moreTypeRaw === 1 || moreTypeRaw === 2 ? moreTypeRaw : 2; // 老语义：首页模块缺省即倒计时
  const titleColor = title.titleColor || '#1a1a1a';
  const descColor = title.descColor || '#1a1a1a';
  const moreColor = title.moreColor || '#1a1a1a';
  const style = Number(title.titleStyle) || 1;
  const customLabel = String(title.countdownLabel || '').trim();
  let countdownLabel = customLabel || fallbackLabel;
  if (!customLabel && countdownTargetTs) {
    if (activityStartTs && activityStartTs > now) countdownLabel = t('距开始');
    else if (countdownTargetTs - now <= 0) countdownLabel = t('已结束');
  }
  const barStyle: Record<string, any> = {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    boxSizing: 'border-box',
    padding: `${Number(title.titlePaddingTop) || 20}px ${innerPaddingPx}px ${Number(title.titlePaddingBottom) || 20}px`,
  };
  const bg1 = String(title.titleBackground || '');
  const bg2 = String(title.titleBackground2 || '');
  if (bg1 && bg2) barStyle.background = `linear-gradient(to right, ${bg1}, ${bg2})`;
  else if (bg1) barStyle.background = bg1;
  if (Number(title.titleRadius)) {
    barStyle.borderTopLeftRadius = `${Number(title.titleRadius)}px`;
    barStyle.borderTopRightRadius = `${Number(title.titleRadius)}px`;
  }
  return (
    <div style={barStyle}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 4 }}>
        {style === 3 && <div style={{ width: 4, height: 16, backgroundColor: '#1a1a1a', borderRadius: 8, alignSelf: 'center' }} />}
        {title.titleBackgroundPic?.picUrl ? <img src={title.titleBackgroundPic.picUrl} alt="" style={{ width: 30, height: 30, marginLeft: 10, objectFit: 'contain' }} /> : null}
        <div style={{ fontSize: 24, fontWeight: 700, color: titleColor, textTransform: 'capitalize' }}>{title.titleText || fallbackText}</div>
        {title.descText ? <div style={{ fontSize: style === 2 ? 16 : 12, color: descColor }}>{title.descText}</div> : null}
      </div>
      {showMore && mode === 2 && <TitleCountdownBox label={countdownLabel} targetTs={countdownTargetTs} now={now} color={moreColor} />}
      {showMore && mode === 1 && (
        <div style={{ display: 'flex', alignItems: 'center', fontSize: 14, color: moreColor, cursor: 'pointer' }}>
          <span>{String(title.moreLink?.title || '').trim() || t('更多')}</span>
          <span style={{ marginLeft: 4, color: 'inherit' }}><TitleMoreArrow /></span>
        </div>
      )}
    </div>
  );
}

/** 轮播左右切换按钮（对齐原版 swiper-button-prev/next 圆角半透明块 + SVG mask 箭头） */
function SwiperArrowButton({ dir, left, right }: { dir: 'prev' | 'next'; left?: number | string; right?: number | string }) {
  return (
    <div
      className={`pc-decorate-swiper-arrow pc-decorate-swiper-arrow--${dir}`}
      style={{ left, right }}
    >
      <span />
    </div>
  );
}





/** 营销通栏画布预览（1:1 对齐原版 Preview.vue：API拉类目/分组商品 + 右面板逐像素还原） */




function HeaderColorInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const { t } = useI18n();
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 4, width: '100%' }}>
      <a style={{ fontSize: 12, cursor: 'pointer', color: 'var(--semi-color-primary)', flexShrink: 0, whiteSpace: 'nowrap' }} onClick={() => onChange('')}>{t('重置')}</a>
      <AlphaColorInput value={value} onChange={onChange} />
    </span>
  );
}

/** 左列类目：优先取 pcCatFloor/list（PC 分类抽屉）真实数据，回退到配置项 */
function useHeroLeftCategories(configCategories: PicItem[], showCount: number) {
  const { t } = useI18n();
  const [apiCategories, setApiCategories] = useState<PicItem[]>([]);
  useEffect(() => {
    let alive = true;
    apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>> }>('/decorate/pcCatFloor/list', {
      page: 1, size: 50, sortField: 'sortOrder', sortOrder: 'asc',
    })
      .then((res) => {
        if (!alive) return;
        const raw = res.records || res.list || [];
        const mapped: PicItem[] = raw.map((item: any) => ({
          picId: String(item.picId ?? item.id ?? ''),
          picUrl: item.picUrl || item.imageUrl || '',
          picThumb: item.picThumb || item.picUrl || item.imageUrl || '',
          picName: item.picName || '',
          picTitle: item.picTitle || item.title || tl('类目'),
          picDesc: '',
          picLink: { link: item.picLink?.link || item.link || '', name: '' },
        }));
        setApiCategories(mapped);
      })
      .catch(() => {
        if (alive) setApiCategories([]);
      });
    return () => {
      alive = false;
    };
  }, []);
  const categories = apiCategories.length ? apiCategories : configCategories;
  return categories.slice(0, Math.max(0, Math.floor(showCount)));
}

/** 卡位商品来源为 productGroup 时：分组详情拿 productIds → 按 ids 查商品（取前 2） */
function useHeroGroupProducts(cards: Array<Record<string, any>>) {
  const [groupProductsMap, setGroupProductsMap] = useState<Record<number, PreviewProduct[]>>({});
  const key = JSON.stringify(cards.map((c) => ({ src: c.productSource, gid: c.productGroupId })));
  useEffect(() => {
    const pending: Array<{ idx: number; gid: string }> = [];
    cards.forEach((card, idx) => {
      if (card.productSource === 'productGroup' && card.productGroupId) {
        pending.push({ idx, gid: String(card.productGroupId) });
      }
    });
    if (!pending.length) {
      setGroupProductsMap({});
      return;
    }
    let alive = true;
    Promise.all(
      pending.map(async ({ idx, gid }) => {
        try {
          const group = await apiGet<Record<string, any>>('/product/productGroup/detail', { id: gid });
          const productIds = Array.isArray(group?.productIds) ? (group.productIds as Array<number | string>).slice(0, 2) : [];
          if (!productIds.length) return [idx, []] as const;
          const result = await apiGet<{ records?: PreviewProduct[] }>('/product/product/list', {
            ids: productIds.join(','), size: 2, page: 1,
          });
          const records = (result.records || []).map((item) => ({
            productId: item.productId,
            productName: item.productName || '',
            picUrl: item.picUrl || '',
            picThumb: item.picThumb || '',
            productPrice: item.productPrice ? `¥${item.productPrice}` : '¥0',
          }));
          return [idx, records] as const;
        } catch {
          return [idx, []] as const;
        }
      }),
    ).then((entries) => {
      if (!alive) return;
      const map: Record<number, PreviewProduct[]> = {};
      entries.forEach(([idx, rows]) => {
        map[idx] = rows as PreviewProduct[];
      });
      setGroupProductsMap(map);
    });
    return () => {
      alive = false;
    };
  }, [key]);
  return groupProductsMap;
}

function HeroPreview({ data }: { data: Record<string, any> }) {
  const { t } = useI18n();
  const m = data || {};
  const frame = m.frame || {};
  const style = m.style || {};
  const left = m.left || {};
  const right = m.right || {};
  const center = m.center || {};

  const gap = Number(style.gap) || 16;
  const height = Number(style.height) || 336;
  const panelShadow = Number(style.panelShadow) === 1 ? '0 2px 12px 0 rgba(0,0,0,0.06)' : 'none';
  const panelBg = style.panelBackgroundColor || '#ffffff';
  const panelRadius = Number(style.panelRadius) || 8;
  const titleSize = Number(style.titleFontSize) || 14;
  const titleColor = style.titleColor || '#333333';
  const mutedColor = style.mutedColor || '#999999';
  const priceColor = style.priceColor || '#e1251b';
  const cardRadius = Number(style.cardRadius) || 12;
  const bannerRadius = Number(style.bannerRadius) || 12;

  const gradientDirMap: Record<string, string> = { upDown: 'to bottom', leftRight: 'to right', diagonal: 'to bottom right' };
  const userGradient =
    right.userCardGradientColorA && right.userCardGradientColorB
      ? `linear-gradient(${gradientDirMap[right.userCardGradientType || 'upDown'] || 'to bottom'}, ${right.userCardGradientColorA}, ${right.userCardGradientColorB})`
      : 'none';

  const showCount = Number(left.showCount) || 10;
  const configCategories = ((left.categories || []) as PicItem[]);
  const categories = useHeroLeftCategories(configCategories, showCount);

  const allModules: Array<Record<string, any>> = (center.modules || []).length ? center.modules : [];
  const promoCards = allModules.slice(0, 2);
  const channelCards = allModules.slice(2, 6);
  const banners = (center.banners || []) as PicItem[];
  const shortcuts = (right.shortcuts || []) as PicItem[];

  const groupProductsMap = useHeroGroupProducts(allModules);

  const panelStyle: Record<string, any> = { background: panelBg, borderRadius: panelRadius, boxShadow: panelShadow, overflow: 'hidden' };
  const cardStyle: Record<string, any> = { ...panelStyle, borderRadius: cardRadius };

  const subtitleBg = (card: Record<string, any>) =>
    `linear-gradient(${gradientDirMap[card.moduleSubTitleBgType || 'leftRight'] || 'to right'}, ${card.moduleSubTitleBgColorA || '#FF4E4E'}, ${card.moduleSubTitleBgColorB || '#E1251B'})`;

  /** 卡位商品渲染：productGroup 来源走 groupProductsMap，products 来源走 card.products */
  const getCardProducts = (card: Record<string, any>, idx: number): PreviewProduct[] => {
    if (card.productSource === 'productGroup') return groupProductsMap[idx] || [];
    return ((card.products || []) as Array<Record<string, any>>).map((p) => ({
      productId: 0,
      productName: '',
      picUrl: p.pic?.picUrl || '',
      picThumb: p.pic?.picThumb || p.pic?.picUrl || '',
      productPrice: p.price || '¥0',
    }));
  };

  const HeroCard = ({ card, cardIndex }: { card: Record<string, any>; cardIndex: number }) => {
    const products = getCardProducts(card, cardIndex);
    return (
      <div style={{ ...cardStyle, width: '100%', height: '100%', padding: 8, display: 'flex', flexDirection: 'column', gap: 6, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 4 }}>
          <span style={{ fontSize: titleSize, fontWeight: 700, color: titleColor, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {card.moduleTitle || t('推荐')}
          </span>
          {card.moduleSubTitle ? (
            <span style={{ fontSize: 10, color: '#fff', background: subtitleBg(card), borderRadius: 3, padding: '1px 4px', flexShrink: 0 }}>
              {card.moduleSubTitle}
            </span>
          ) : null}
        </div>
        <div style={{ flex: 1, display: 'flex', gap: 6, minHeight: 0 }}>
          {products.length
            ? products.slice(0, 2).map((p, pi) => {
                const rawP = ((card.products || [])[pi] || {}) as Record<string, any>;
                return (
                  <div key={pi} style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}>
                    <div style={{ flex: 1, borderRadius: 6, overflow: 'hidden', background: '#f7f8fa', position: 'relative', minHeight: 0 }}>
                      {(p.picThumb || p.picUrl) ? <img src={p.picThumb || p.picUrl} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : null}
                      {rawP.label ? (
                        <span
                          style={{
                            position: 'absolute', left: 0, bottom: 0, fontSize: 9, color: '#fff', padding: '1px 4px',
                            background: `linear-gradient(to right, ${rawP.labelBgColorA || '#FF4E4E'}, ${rawP.labelBgColorB || '#E1251B'})`,
                          }}
                        >
                          {rawP.label}
                        </span>
                      ) : null}
                    </div>
                    {String(card.showPrice) !== '0' && (
                      <div style={{ fontSize: 12, fontWeight: 700, color: priceColor, textAlign: 'center' }}>
                        {typeof p.productPrice === 'string' && p.productPrice.startsWith('¥') ? p.productPrice : `¥${Number(p.productPrice ?? 0).toFixed(2)}`}
                      </div>
                    )}
                  </div>
                );
              })
            : <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: mutedColor, fontSize: 12 }}>{t('配置商品')}</div>}
        </div>
      </div>
    );
  };

  const framePad = `${Number(frame.boxPaddingTop) || 0}px ${Number(frame.boxPadding) || 0}px ${Number(frame.boxPaddingBottom) || 0}px`;

  return (
    <div style={{ padding: framePad, background: frame.backgroundColor || 'transparent', borderRadius: Number(frame.boxRadius) || 0 }}>
      <div style={{ display: 'grid', gridTemplateColumns: `${style.leftWidth || 220}px minmax(0, 1fr) ${style.rightWidth || 260}px`, gap, height, overflow: 'hidden', alignItems: 'stretch' }}>

        {/* 左：类目列表（API 优先） */}
        <section style={{ ...panelStyle, background: left.backgroundColor || '#ffffff', color: left.textColor || '#333333', paddingTop: 6, overflowY: 'auto' }}>
          {categories.length ? (
            categories.map((c, i) => (
              <div key={c.picId ?? i} style={{ display: 'flex', alignItems: 'center', gap: 10, minHeight: 28, padding: '4px 8px 4px 16px', fontSize: 12 }}>
                {c.picUrl ? <img src={c.picUrl} alt="" style={{ width: 18, height: 18, flexShrink: 0 }} /> : <span style={{ width: 12, color: 'inherit' }}>-</span>}
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.picTitle || t('类目')}</span>
              </div>
            ))
          ) : (
            <div style={{ padding: 30, textAlign: 'center', color: mutedColor, fontSize: 12 }}>{t('点击配置左侧类目')}</div>
          )}
        </section>

        {/* 中：上（轮播+促销卡）/ 下（4 频道卡） */}
        <section style={{ display: 'flex', flexDirection: 'column', gap, minWidth: 0, minHeight: 0 }}>
          <div style={{ flex: 1, display: 'flex', gap, minHeight: 0 }}>
            <div style={{ ...panelStyle, flex: 1, borderRadius: bannerRadius, minWidth: 0 }}>
              {banners.length ? (
                <img src={banners[0]?.picUrl} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }} />
              ) : (
                <div style={{ padding: 20, textAlign: 'center', color: mutedColor, fontSize: 12 }}>{t('点击添加轮播')}</div>
              )}
            </div>
            <div style={{ flex: 1, display: 'flex', gap, minWidth: 0 }}>
              {promoCards.map((card, i) => (
                <div key={i} style={{ flex: 1, minWidth: 0 }}>
                  <HeroCard card={card} cardIndex={i} />
                </div>
              ))}
            </div>
          </div>
          <div style={{ flex: 1, minHeight: 0 }}>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, minmax(0, 1fr))', gap, height: '100%' }}>
              {channelCards.map((card, i) => (
                <HeroCard key={i} card={card} cardIndex={i + 2} />
              ))}
            </div>
          </div>
        </section>

        {/* 右：用户卡 + 店铺街（1:1 还原） */}
        <section style={{ display: 'flex', flexDirection: 'column', gap, minWidth: 0, minHeight: 0 }}>
          {/* 用户卡 */}
          <div style={{ ...panelStyle, height: 220, flexShrink: 0, padding: '16px 12px', textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', background: userGradient !== 'none' ? userGradient : right.userCardBackground || panelBg, position: 'relative' }}>
            {/* 付费会员入口：绝对定位右上角 */}
            {right.premiumMember?.show === true && (
              <div style={{ position: 'absolute', top: 0, right: 0, zIndex: 2, width: 86, height: 41, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', background: '#1a1a1a', color: '#ffd700', borderRadius: '0 0 0 12px', fontSize: 12, fontWeight: 700 }}>
                {right.premiumMember.title || t('会员')}
                {right.premiumMember.description ? <span style={{ fontWeight: 400, fontSize: 9 }}>{right.premiumMember.description}</span> : null}
              </div>
            )}
            {/* 用户信息行 */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, width: '100%', marginBottom: 15 }}>
              <img src={getPath(right, 'user.avatar.picUrl') || ''} alt="" style={{ width: 36, height: 36, borderRadius: '50%', background: '#ececec', flexShrink: 0 }} />
              <div style={{ flex: 1, minWidth: 0, textAlign: 'left' }}>
                <div style={{ fontSize: 16, fontWeight: 700, color: '#333', paddingBottom: 4, whiteSpace: 'nowrap', overflow: 'hidden' }}>
                  {getPath(right, 'user.nickname') || t('欢迎来商城')}
                </div>
                <div style={{ fontSize: 12, color: '#666', display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span>{t('注册')}</span>
                  <span style={{ width: 1, height: 10, background: '#ddd' }} />
                  <span>{t('入驻')}</span>
                </div>
              </div>
            </div>
            {/* 欢迎语 */}
            <div style={{ width: '100%', textAlign: 'center', marginBottom: 8 }}>
              <div style={{ fontSize: 15, fontWeight: 700, color: '#333', marginBottom: 4 }}>{getPath(right, 'user.welcomeText') || t('登录后更多精彩')}</div>
              <div style={{ fontSize: 12, color: '#999' }}>{t('一键登录，福利好物全拥有')}</div>
            </div>
            {/* 立即登录按钮 */}
            <div style={{ display: 'block', width: '100%', height: 36, lineHeight: '36px', background: '#ff4141', color: '#fff', fontSize: 14, borderRadius: 4, textAlign: 'center', flexShrink: 0 }}>
              {t('立即登录')}
            </div>
            {/* 四宫格动作 */}
            <div style={{ width: '100%', display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 2, marginTop: 10, flexShrink: 0 }}>
              {[t('我的订单'), t('浏览记录'), t('商品收藏'), t('关注店铺')].map((label) => (
                <div key={label} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4, fontSize: 11, color: '#333' }}>
                  <span style={{ width: 20, height: 20, borderRadius: '50%', background: '#f2f3f5', display: 'inline-block' }} />
                  <span>{label}</span>
                </div>
              ))}
            </div>
          </div>
          {/* 店铺街 */}
          <div style={{ ...panelStyle, flex: 1, padding: '0 12px 10px', display: 'flex', flexDirection: 'column', minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', margin: '10px 0', flexShrink: 0 }}>
              <div style={{ fontWeight: 700, fontSize: titleSize, color: titleColor, display: 'flex', alignItems: 'center', gap: 4 }}>
                {getPath(right, 'shopsTitle') || t('店铺街')}
                {getPath(right, 'shopsSubTitle') ? (
                  <span style={{ fontSize: 10, color: '#fff', background: getPath(right, 'shopsSubTitleBg') || '#FF4E4E', borderRadius: 3, padding: '1px 4px' }}>
                    {getPath(right, 'shopsSubTitle')}
                  </span>
                ) : null}
              </div>
              <a style={{ fontSize: 11, color: mutedColor }}>
                {getPath(right, 'shopsMoreText') || t('查看全部')} <span style={{ fontSize: 9 }}>▾</span>
              </a>
            </div>
            {shortcuts.length ? (
              <div style={{ flex: 1, display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 6, alignContent: 'start' }}>
                {shortcuts.map((sc, i) => (
                  <div key={sc.picId ?? i} style={{ textAlign: 'center' }}>
                    <div style={{ width: 44, height: 44, borderRadius: 8, overflow: 'hidden', background: '#ffffff', border: '1px solid #f2f2f2', margin: '0 auto', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
                      {sc.picUrl ? <img src={sc.picUrl} alt="" style={{ width: '100%', height: '100%', objectFit: 'contain' }} /> : null}
                    </div>
                    <span style={{ fontSize: 11, color: getPath(right, 'shopsTextColor') || '#a06a38', display: 'block', marginTop: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {sc.picTitle || t('进入店铺')}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', color: mutedColor, fontSize: 12 }}>{t('点击配置店铺')}</div>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}


const DEFAULT_PC_HEADER_LOGO = `${import.meta.env.BASE_URL}admin-assets/admin-light-logo.jpeg`;

const PC_HEADER_DEFAULTS: Record<string, any> = {
  styleType: 1,
  isWhite: 1,
  bottomDividerType: 1,
  showTopMenuBar: 1,
  topMenuTextColor: '',
  topMenuTextHoverColor: '',
  showTopRegion: 1,
  showTopUserMenu: 1,
  showTopMemberCenter: 1,
  memberCenterItems: [
    { picTitle: '我的足迹', picDesc: '', picLink: { link: '/member/browsingHistory/list', name: '我的足迹', label: '我的足迹' } },
    { picTitle: '我的优惠券', picDesc: '', picLink: { link: '/member/coupon/list', name: '我的优惠券', label: '我的优惠券' } },
  ],
  showTopMemberName: 1,
  showTopMessage: 1,
  showTopOrders: 1,
  showTopSiteNav: 1,
  showTopAd: 1,
  topAdPic: { picUrl: '', picThumb: '' },
  logoPic: { picUrl: '', picThumb: '' },
  logoAlign: 1,
  logoWidth: 100,
  logoHeight: 25,
  rightCartShow: 1,
  cartStyle: 1,
  cartPic: { picUrl: '', picThumb: '' },
  topMenuCartShow: 1,
  searchStyle: 1,
  searchColorType: 1,
  searchColor: '',
  showCategory: 1,
  showHomeCategory: 1,
  categoryMarginTop: 0,
  showMainNav: 1,
  mainNavPaddingTop: 0,
  mainNavPaddingBottom: 0,
  showBenefitBar: 1,
  benefitBarBackgroundColor: '',
  benefitTitleColor: '',
  benefitDescColor: '',
  benefitNoticeGroups: [],
};

function BenefitNoticePreview({
  notices,
  titleColor,
  descColor,
}: {
  notices: Array<Record<string, any>>;
  titleColor?: string;
  descColor?: string;
}) {
  const [index, setIndex] = useState(0);
  const [textWidth, setTextWidth] = useState(0);
  const measureRef = useRef<HTMLDivElement>(null);
  const signature = notices.map((notice) => `${notice.picUrl || ''}|${notice.picTitle || ''}|${notice.picDesc || ''}`).join('\u0001');

  useEffect(() => {
    setIndex(0);
    if (notices.length <= 1) return undefined;
    const timer = window.setInterval(() => {
      setIndex((current) => (current + 1) % notices.length);
    }, 3000);
    return () => window.clearInterval(timer);
    // 仅在提示内容变化时重启轮播，颜色变化不应重置
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);

  useEffect(() => {
    const root = measureRef.current;
    if (!root) return;
    const widths = Array.from(root.querySelectorAll<HTMLElement>('.pchead-benefit__measure-item'))
      .map((el) => Math.ceil(el.offsetWidth || el.scrollWidth));
    setTextWidth(widths.length ? Math.max(...widths) : 0);
  }, [signature]);

  const notice = notices[index] || notices[0];
  if (!notice) return null;

  return (
    <div className={`pchead-benefit${notices.length > 1 ? ' pchead-benefit--carousel' : ''}`}>
      <div className="pchead-benefit__measure" ref={measureRef} aria-hidden="true">
        {notices.map((item, itemIndex) => (
          <span className="pchead-benefit__text pchead-benefit__measure-item" key={itemIndex}>
            {item.picTitle && <strong>{item.picTitle}</strong>}
            {item.picDesc && <small>{item.picDesc}</small>}
          </span>
        ))}
      </div>
      {notice.picUrl && <img className="pchead-benefit__icon" src={notice.picUrl} alt="" />}
      {(notice.picTitle || notice.picDesc) && (
        <span className="pchead-benefit__text" style={textWidth > 0 ? { minWidth: textWidth } : undefined}>
          <span className="pchead-benefit__slide" key={`${index}-${notice.picTitle || ''}-${notice.picDesc || ''}`}>
            {notice.picTitle && (
              <strong style={titleColor ? { color: titleColor } : undefined}>{notice.picTitle}</strong>
            )}
            {notice.picDesc && (
              <small style={descColor ? { color: descColor } : undefined}>{notice.picDesc}</small>
            )}
          </span>
        </span>
      )}
    </div>
  );
}

function PcHeaderPreviewStrip({ header, onOpen, regionText, active }: { header: Record<string, any>; onOpen: () => void; regionText: string; active?: boolean }) {
  const { t } = useI18n();
  const h = { ...PC_HEADER_DEFAULTS, ...header };
  const styleType = Number(h.styleType ?? 1) === 2 ? 2 : 1;
  const simple = styleType === 1;
  const showBenefitBar = Number(h.showBenefitBar ?? 1) !== 0;
  const showTopMenuBar = Number(h.showTopMenuBar ?? 1) !== 0;
  const showTopRegion = Number(h.showTopRegion ?? 1) !== 0;
  const showTopUserMenu = Number(h.showTopUserMenu ?? 1) !== 0;
  const showTopMemberCenter = Number(h.showTopMemberCenter ?? 1) !== 0;
  const showTopMemberName = Number(h.showTopMemberName ?? 1) !== 0;
  const showTopMessage = Number(h.showTopMessage ?? 1) !== 0;
  const showTopOrders = Number(h.showTopOrders ?? 1) !== 0;
  const showTopSiteNav = Number(h.showTopSiteNav ?? 1) !== 0;
  const showRightCart = Number(h.rightCartShow ?? 1) !== 0;
  const cartStyle = Number(h.cartStyle ?? 1);
  const showCategory = Number(h.showCategory ?? 1) !== 0 && Number(h.showHomeCategory ?? 1) !== 0;
  const showMainNav = Number(h.showMainNav ?? 1) !== 0;
  const showTopMenuCart = Number(h.topMenuCartShow ?? 1) !== 0;
  const divider = [1, 2].includes(Number(h.bottomDividerType)) ? Number(h.bottomDividerType) : simple ? 2 : 1;
  const navTitles = useHeaderNavTitles();
  const navItems = navTitles.length ? navTitles : [t('首页'), t('导航1'), t('导航2')];

  const benefitGroups = ((h.benefitNoticeGroups || []) as Array<Record<string, any>>)
    .map((group) => ({
      ...group,
      notices: ((group.notices || []) as Array<Record<string, any>>).filter((notice) => (
        Boolean(notice?.picUrl || notice?.picTitle || notice?.picDesc || notice?.picLink?.link)
      )),
    }))
    .filter((group) => group.notices.length > 0);

  const topMenuTextStyle: Record<string, any> = {};
  if (h.topMenuTextColor) {
    topMenuTextStyle.color = h.topMenuTextColor;
    (topMenuTextStyle as any)['--pchead-strip-color'] = h.topMenuTextColor;
  }
  if (h.topMenuTextHoverColor) (topMenuTextStyle as any)['--hover-color'] = h.topMenuTextHoverColor;

  const stripLeft = simple && (showTopRegion || showTopMemberName);
  const stripRight = showTopSiteNav || showTopMenuCart || (simple && showTopMenuBar && showTopUserMenu);

  const logoStyle: Record<string, any> = {
    width: Math.min(Number(h.logoWidth) || 100, 160),
    height: Math.min(Number(h.logoHeight) || 25, 60),
    objectFit: 'contain',
  };
  const logoAlign = Number(h.logoAlign ?? 1);
  const searchAccent = Number(h.searchColorType ?? 1) === 2 && h.searchColor ? h.searchColor : '#1a1a1a';

  return (
    <div className={`dec-pc-header-preview${active ? ' dec-pc-header-preview--active' : ''}`} onClick={onOpen} title={t('点击配置 PC 头部装修')}>
      {/* 顶部提示条（showBenefitBar 总开关，两种风格共用） */}
      {showBenefitBar && (
        <div className="dec-pc-header-preview__strip" style={h.benefitBarBackgroundColor ? { background: h.benefitBarBackgroundColor } : undefined}>
          <div className="dec-pc-header-preview__strip-inner">
            <div className={`pchead-strip${simple ? '' : ' pchead-strip--overseas'}`}>
              <div className="pchead-strip__left" style={topMenuTextStyle}>
                {simple && showTopRegion && <span className="pchead-strip__region"><span className="pchead-strip__region-ico"><Local size={14} /></span>{regionText || t('全国')}</span>}
                {simple && showTopMemberName && <a className="pchead-strip__link">{t('会员信息')}</a>}
              </div>
              {!simple && benefitGroups.length > 0 && (
                <div className="pchead-strip__benefits">
                  {benefitGroups.map((group, groupIndex) => {
                    return (
                      <BenefitNoticePreview
                        key={groupIndex}
                        notices={group.notices}
                        titleColor={h.benefitTitleColor}
                        descColor={h.benefitDescColor}
                      />
                    );
                  })}
                </div>
              )}
              <div className="pchead-strip__right" style={topMenuTextStyle}>
                {showTopMenuCart && <a className="pchead-strip__link">🛒 {t('购物车')}</a>}
                {simple && showTopMenuBar && showTopUserMenu && (
                  <span className="pchead-strip__menus">
                    {showTopMemberCenter && <a className="pchead-strip__link">{t('会员中心')} ▾</a>}
                    {showTopMessage && <a className="pchead-strip__link">{t('站内消息')}</a>}
                    {showTopOrders && <a className="pchead-strip__link">{t('我的订单')}</a>}
                  </span>
                )}
                {showTopSiteNav && <a className="pchead-strip__link">{t('站点导航')} ▾</a>}
              </div>
            </div>
          </div>
        </div>
      )}
      {/* 头部主体（简约风） */}
      {simple && (
        <div className={`dec-pchead-body${Number(h.isWhite ?? 1) !== 0 ? ' dec-pchead-body--white' : ''} dec-pchead-body--divider-${divider}`}>
          <div className="dec-pchead-body__grid">
            <div className="dec-pchead-body__left">
              <div className={`dec-pchead-logo dec-pchead-logo--${logoAlign === 2 ? 'center' : logoAlign === 3 ? 'right' : 'left'}`}>
                <img src={h.logoPic?.picUrl || DEFAULT_PC_HEADER_LOGO} alt="logo" style={logoStyle} />
              </div>
              {showCategory && (
                <div className="dec-pchead-cate" style={{ marginTop: Number(h.categoryMarginTop) || 0 }}>
                  <span className="dec-pchead-cate__icon">☰</span>
                  <span className="dec-pchead-cate__txt">{t('分类')}</span>
                  <span className="dec-pchead-cate__arrow">▾</span>
                </div>
              )}
            </div>
            <div className="dec-pchead-body__center">
              <div className="dec-pchead-search">
                <div className="dec-pchead-search__box" style={{ borderColor: searchAccent }}>
                  <span className="dec-pchead-search__type">{t('商品')} <i>▾</i></span>
                  <input className="dec-pchead-search__input" placeholder={t('请输入关键词')} disabled />
                  <span className="dec-pchead-search__submit" style={{ background: searchAccent }}>{t('搜索')}</span>
                </div>
                {showMainNav && (
                  <ul className="dec-pchead-nav" style={{ marginTop: Number(h.mainNavPaddingTop) || 0, marginBottom: Number(h.mainNavPaddingBottom) || 0 }}>
                    {navItems.map((label: string, i: number) => <li key={i}>{label}</li>)}
                  </ul>
                )}
              </div>
            </div>
            <div className="dec-pchead-body__right">
              {showRightCart && (
                cartStyle === 2 ? (
                  <div className={`dec-pchead-cartpic${h.cartPic?.picUrl ? '' : ' dec-pchead-cartpic--empty'}`}>
                    {h.cartPic?.picUrl ? <img src={h.cartPic.picUrl} alt="" /> : <span>{t('购物车图')}</span>}
                  </div>
                ) : (
                  <div className="dec-pchead-cart">
                    <span className="dec-pchead-cart__icon">🛒</span>
                    <span className="dec-pchead-cart__count">18</span>
                    <span className="dec-pchead-cart__text">{t('购物车')}</span>
                  </div>
                )
              )}
            </div>
          </div>
        </div>
      )}
      {!simple && (
        <div className={`dec-pchead-overseas dec-pchead-overseas--divider-${divider}`}>
          <div className="dec-pchead-overseas__grid">
            <div className="dec-pchead-body__left">
              <div className={`dec-pchead-logo dec-pchead-logo--${logoAlign === 2 ? 'center' : logoAlign === 3 ? 'right' : 'left'}`}>
                <img src={h.logoPic?.picUrl || DEFAULT_PC_HEADER_LOGO} alt="logo" style={logoStyle} />
              </div>
              {showCategory && (
                <div className="dec-pchead-cate" style={{ marginTop: Number(h.categoryMarginTop) || 0 }}>
                  <span className="dec-pchead-cate__icon">☰</span>
                  <span className="dec-pchead-cate__txt">{t('分类')}</span>
                  <span className="dec-pchead-cate__arrow">▾</span>
                </div>
              )}
            </div>
            <div className="dec-pchead-body__center">
              <div className="dec-pchead-search">
                <div className="dec-pchead-search__box" style={{ borderColor: searchAccent }}>
                  <span className="dec-pchead-search__type">{t('商品')} <i>▾</i></span>
                  <input className="dec-pchead-search__input" placeholder={t('请输入关键词')} disabled />
                  <span className="dec-pchead-search__submit" style={{ background: searchAccent }}>{t('搜索')}</span>
                </div>
                {showMainNav && (
                  <ul className="dec-pchead-nav" style={{ marginTop: Number(h.mainNavPaddingTop) || 0, marginBottom: Number(h.mainNavPaddingBottom) || 0 }}>
                    {navItems.map((label: string, i: number) => <li key={i}>{label}</li>)}
                  </ul>
                )}
              </div>
            </div>
            <div className="dec-pchead-overseas__right">
              <div className="dec-pchead-overseas__entry">
                <IconGlobe />
                <span>EN</span>
                <i>▾</i>
              </div>
              <div className="dec-pchead-overseas__entry">
                <IconUser />
                <span>{t('我的')}</span>
                <i>▾</i>
              </div>
              {showRightCart && (
                cartStyle === 2 ? (
                  <div className={`dec-pchead-cartpic${h.cartPic?.picUrl ? '' : ' dec-pchead-cartpic--empty'}`}>
                    {h.cartPic?.picUrl ? <img src={h.cartPic.picUrl} alt="" /> : <span>{t('购物车图')}</span>}
                  </div>
                ) : (
                  <div className="dec-pchead-cart">
                    <span className="dec-pchead-cart__icon">🛒</span>
                    <span className="dec-pchead-cart__count">18</span>
                    <span className="dec-pchead-cart__text">{t('购物车')}</span>
                  </div>
                )
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}


function PageSettingsPanel({
  pageModule,
  legacyCount,
  newCount,
  onChange,
  onSwitchVersion,
}: {
  pageModule: Record<string, any>;
  legacyCount: number;
  newCount: number;
  onChange: (next: Record<string, any>) => void;
  onSwitchVersion: (target: 'new' | 'old') => void;
}) {
  const { t } = useI18n();
  const currentVersion = pageModule.homeStyleVersion === 'old' ? 'old' : 'new';
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <Text strong>{t('页面设置')}</Text>

      <div>
        <Text type="tertiary" size="small">{t('页面标题（可不填，默认为页面名称）')}</Text>
        <Input value={pageModule.title || ''} onChange={(v) => onChange({ ...pageModule, title: v })} placeholder={t('请输入页面标题')} />
      </div>

      {/* 头部风格仅服务旧版头部；新版 PC 首页走 pcHeader，故新版时隐藏（原版同逻辑） */}
      {currentVersion === 'old' && (
        <div>
          <Text type="tertiary" size="small">{t('头部风格')}</Text>
          <RadioGroup
            value={Number(pageModule.headerStyle) === 2 ? 2 : 1}
            onChange={(e: any) => onChange({ ...pageModule, headerStyle: Number(e.target.value) })}
            type="button"
          >
            <Radio value={1}>{t('默认风格')}</Radio>
            <Radio value={2}>{t('简约风格')}</Radio>
          </RadioGroup>
        </div>
      )}

      <div>
        <Text type="tertiary" size="small">{t('标题栏背景颜色')}</Text>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <AlphaColorInput value={pageModule.titleBackgroundColor || ''} onChange={(v) => onChange({ ...pageModule, titleBackgroundColor: v })} />
          <Button size="small" onClick={() => onChange({ ...pageModule, titleBackgroundColor: '' })}>{t('重置')}</Button>
        </div>
      </div>

      <div>
        <Text type="tertiary" size="small">{t('页面背景颜色')}</Text>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <AlphaColorInput value={pageModule.backgroundColor || ''} onChange={(v) => onChange({ ...pageModule, backgroundColor: v })} />
          <Button size="small" onClick={() => onChange({ ...pageModule, backgroundColor: '' })}>{t('重置')}</Button>
        </div>
      </div>

      <div>
        <Text type="tertiary" size="small">{t('背景图')}</Text>
        <Input
          value={typeof pageModule.backgroundImage === 'string' ? pageModule.backgroundImage : pageModule.backgroundImage?.picUrl || ''}
          onChange={(v) => onChange({ ...pageModule, backgroundImage: v ? { picUrl: v, picThumb: v } : { picUrl: '', picThumb: '' } })}
          placeholder="https://..."
        />
      </div>

      <div>
        <Text type="tertiary" size="small">{t('背景图显示')}</Text>
        <RadioGroup
          value={Number(pageModule.backgroundRepeat) || 1}
          onChange={(e: any) => onChange({ ...pageModule, backgroundRepeat: Number(e.target.value) })}
          type="button"
        >
          <Radio value={1}>{t('不平铺')}</Radio>
          <Radio value={2}>{t('平铺')}</Radio>
          <Radio value={3}>{t('纵向平铺')}</Radio>
          <Radio value={4}>{t('横向平铺')}</Radio>
        </RadioGroup>
      </div>

      <div>
        <Text type="tertiary" size="small">{t('背景图填充大小')}</Text>
        <RadioGroup
          value={Number(pageModule.backgroundSize) || 1}
          onChange={(e: any) => onChange({ ...pageModule, backgroundSize: Number(e.target.value) })}
          type="button"
        >
          <Radio value={1}>{t('横向100%')}</Radio>
          <Radio value={2}>{t('纵向100%')}</Radio>
          <Radio value={3}>{t('100%填充')}</Radio>
        </RadioGroup>
      </div>

      <div style={{ borderTop: '1px solid var(--semi-color-border)', paddingTop: 12 }}>
        <Text type="tertiary" size="small">{t('首页装修版本')}</Text>
        <RadioGroup
          value={currentVersion}
          onChange={(e: any) => onSwitchVersion(e.target.value as 'new' | 'old')}
          type="button"
          style={{ marginTop: 4 }}
        >
          <Radio value="new">{t('新版首页')}</Radio>
          <Radio value="old">{t('旧版首页')}</Radio>
        </RadioGroup>
        <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <Text type="tertiary" size="small"><b>{t('新版首页：')}</b>{t('支持可视化网格、高级轮播等新模块，顶部带 PC 头部实时预览，排版更现代。当前 {count} 个模块。', { count: newCount })}</Text>
          <Text type="tertiary" size="small"><b>{t('旧版首页：')}</b>{t('使用传统全宽占位模块，无头部预览，适合维护旧有页面。当前 {count} 个模块。', { count: legacyCount })}</Text>
        </div>
      </div>
    </div>
  );
}


function HeaderSection({ title, children }: { title: string; children: ReactNode }) {
  const { t } = useI18n();
  return (
    <div style={{ marginTop: 14, borderTop: '1px solid #eef0f3', paddingTop: 12 }}>
      <div style={{ fontSize: 13, fontWeight: 600, color: '#1f2329', marginBottom: 8 }}>{t(title)}</div>
      {children}
    </div>
  );
}

function HeaderField({
  label,
  value,
  showIf = true,
  children,
}: {
  label: string;
  value?: ReactNode;
  showIf?: boolean;
  children?: ReactNode;
}) {
  const { t } = useI18n();
  if (!showIf) return null;
  return (
    <div className="pc-header-field" style={{ display: 'flex', alignItems: 'flex-start', gap: 10, padding: '7px 0' }}>
      <div style={{ width: 118, flexShrink: 0, color: '#646a73', fontSize: 13, lineHeight: '28px' }}>{t(label)}</div>
      <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        {children}
        {value ? <div style={{ color: '#98a2b3', fontSize: 12, lineHeight: '20px', marginTop: 2 }}>{typeof value === 'string' ? t(value) : value}</div> : null}
      </div>
    </div>
  );
}

/* SliderWithInput 已抽到共享 components/SliderWithInput.tsx（实现逐行等价），顶部统一 import。 */

function BenefitGroupsEditor({
  groups,
  onChange,
}: {
  groups: Array<Record<string, any>>;
  onChange: (next: Array<Record<string, any>>) => void;
}) {
  const { t } = useI18n();
  const newNotice = () => ({ picId: '0', picThumb: '', picUrl: '', picName: '', picTitle: '', picDesc: '', picLink: { link: '', name: '' } });
  const updateNotice = (groupIndex: number, noticeIndex: number, patch: Record<string, any>) => {
    onChange(groups.map((group, gi) => (
      gi === groupIndex
        ? { ...group, notices: (group.notices || []).map((notice: any, ni: number) => (ni === noticeIndex ? { ...notice, ...patch } : notice)) }
        : group
    )));
  };
  return (
    <div className="pchead-benefit-groups">
      {groups.map((group, groupIndex) => (
        <div className="pchead-benefit-group" key={groupIndex}>
          <div className="pchead-benefit-group__head">
            <Text strong size="small">{t('第【{index}】组', { index: groupIndex + 1 })}</Text>
            <Tooltip content={t('删除分组')} position="left">
              <Button
                size="small"
                theme="borderless"
                type="danger"
                icon={<IconDelete />}
                onClick={() => {
                  Modal.confirm({
                    title: t('删除分组'),
                    content: t('确定删除第【{index}】组及组内所有提示吗？', { index: groupIndex + 1 }),
                    okText: t('删除'),
                    cancelText: t('取消'),
                    onOk: () => onChange(groups.filter((_, gi) => gi !== groupIndex)),
                  });
                }}
              />
            </Tooltip>
          </div>
          {((group.notices || []) as Array<Record<string, any>>).map((notice, noticeIndex) => (
            <div className="pchead-benefit-notice" key={noticeIndex}>
              <div className="pchead-benefit-notice__head">
                <Text type="tertiary" size="small">{t('提示 {index}', { index: noticeIndex + 1 })}</Text>
                <Tooltip content={t('删除提示')} position="left">
                  <Button
                    size="small"
                    theme="borderless"
                    type="danger"
                    icon={<IconDelete />}
                    onClick={() => onChange(groups.map((grp, gi) => (
                      gi === groupIndex ? { ...grp, notices: (grp.notices || []).filter((_: any, ni: number) => ni !== noticeIndex) } : grp
                    )))}
                  />
                </Tooltip>
              </div>
              <ImageUploader
                single
                value={notice.picUrl ? [notice.picUrl] : []}
                onChange={(urls) => updateNotice(groupIndex, noticeIndex, { picUrl: urls[0] || '', picThumb: urls[0] || '' })}
              />
              <Input
                value={notice.picTitle || ''}
                onChange={(v) => updateNotice(groupIndex, noticeIndex, { picTitle: v })}
                placeholder={t('主标题')}
                size="small"
              />
              <Input
                value={notice.picDesc || ''}
                onChange={(v) => updateNotice(groupIndex, noticeIndex, { picDesc: v })}
                placeholder={t('副标题')}
                size="small"
              />
              <LinkField
                value={notice.picLink}
                onChange={(link) => updateNotice(groupIndex, noticeIndex, { picLink: link })}
              />
            </div>
          ))}
          <Button
            size="small"
            icon={<IconPlus />}
            style={{ marginTop: 8, width: '100%' }}
            onClick={() => onChange(groups.map((grp, gi) => (
              gi === groupIndex ? { ...grp, notices: [...(grp.notices || []), newNotice()] } : grp
            )))}
          >
            {t('添加提示')}
          </Button>
        </div>
      ))}
      <Button size="small" icon={<IconPlus />} style={{ marginTop: 6 }} onClick={() => onChange([...groups, { notices: [newNotice()] }])}>
        {t('添加提示组')}
      </Button>
    </div>
  );
}

function useHeaderNavTitles() {
  const [titles, setTitles] = useState<string[]>([]);
  useEffect(() => {
    let alive = true;
    apiGet<any>('/decorate/pcNavigation/list', { page: 1, size: 10 })
      .then((result) => {
        if (!alive) return;
        const rows = result?.records || result?.list || [];
        const next = rows
          .filter((item: any) => item.isShow !== 0)
          .map((item: any) => String(item.navName || item.name || item.title || ''))
          .filter(Boolean);
        if (next.length) setTitles(next);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);
  return titles;
}

function PcHeaderPanel({
  header,
  onChange,
}: {
  header: Record<string, any>;
  onChange: (next: Record<string, any>) => void;
}) {
  const { t } = useI18n();
  const set = (key: string, value: unknown) => onChange({ ...header, [key]: value });
  const simple = Number(header.styleType ?? 1) === 1;

  const updateMemberItems = (next: Array<Record<string, any>>) => set('memberCenterItems', next);
  const updateBenefitGroups = (next: Array<Record<string, any>>) => set('benefitNoticeGroups', next);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, paddingBottom: 20 }}>
      <Text strong>{t('PC头部装修')}</Text>

      <HeaderField label="头部风格">
        <RadioGroup value={Number(header.styleType ?? 1) === 2 ? 2 : 1} onChange={(e: any) => set('styleType', Number(e.target.value) === 2 ? 2 : 1)} type="button">
          <Radio value={1}>{t('简约风')}</Radio>
          <Radio value={2}>{t('跨境风')}</Radio>
        </RadioGroup>
      </HeaderField>
      <HeaderField label="底部分隔">
        <RadioGroup value={Number(header.bottomDividerType ?? 1)} onChange={(e: any) => set('bottomDividerType', Number(e.target.value))} type="button">
          <Radio value={1}>{t('阴影')}</Radio>
          <Radio value={2}>{t('边框')}</Radio>
          <Radio value={0}>{t('不展示')}</Radio>
        </RadioGroup>
      </HeaderField>

      <HeaderSection title={t('LOGO设置')}>
        <div style={{ width: '100%' }}>
          <HeaderField label="LOGO图片" value="建议 320*120 PNG 透明底">
            <a style={{ fontSize: 12, cursor: 'pointer', color: 'var(--semi-color-primary)' }} onClick={() => set('logoPic', { picUrl: '', picThumb: '' })}>{t('默认logo')}</a>
          </HeaderField>
          <div className="pchead-logo-uploader">
            <ImageUploader
              single
              value={header.logoPic?.picUrl ? [header.logoPic.picUrl] : []}
              onChange={(urls) => set('logoPic', { picUrl: urls[0] || '', picThumb: urls[0] || '' })}
            />
          </div>
          <div className="pchead-logo-default-hint">
            <img src={DEFAULT_PC_HEADER_LOGO} alt="default logo" />
            <span>{t('未上传自定义 LOGO 时使用商城默认 LOGO')}</span>
          </div>
        </div>
        <HeaderField label="logo位置">
          <RadioGroup value={Number(header.logoAlign ?? 1)} onChange={(e: any) => set('logoAlign', Number(e.target.value))} type="button">
            <Radio value={1}>{t('居左')}</Radio>
            <Radio value={2}>{t('居中')}</Radio>
            <Radio value={3}>{t('居右')}</Radio>
          </RadioGroup>
        </HeaderField>
        <HeaderField label="LOGO宽度">
          <SliderWithInput
            value={Number(header.logoWidth) || 100}
            min={10}
            max={160}
            onChange={(v) => set('logoWidth', v)}
          />
        </HeaderField>
        <HeaderField label="LOGO高度">
          <SliderWithInput
            value={Number(header.logoHeight) || 25}
            min={10}
            max={60}
            onChange={(v) => set('logoHeight', v)}
          />
        </HeaderField>
      </HeaderSection>

      <HeaderSection title={t('购物车')}>
        <HeaderField label="右侧购物车">
          <RadioGroup value={Number(header.rightCartShow ?? 1)} onChange={(e: any) => set('rightCartShow', Number(e.target.value))} type="button">
            <Radio value={1}>{t('显示')}</Radio>
            <Radio value={0}>{t('隐藏')}</Radio>
          </RadioGroup>
        </HeaderField>
        <HeaderField label="购物车样式">
          <RadioGroup value={Number(header.cartStyle ?? 1)} onChange={(e: any) => set('cartStyle', Number(e.target.value))} type="button">
            <Radio value={1}>{t('购物车')}</Radio>
            <Radio value={2}>{t('图片')}</Radio>
          </RadioGroup>
        </HeaderField>
        {Number(header.rightCartShow ?? 1) !== 0 && Number(header.cartStyle ?? 1) === 2 && (
          <div style={{ width: '100%' }}>
            <HeaderField label="添加图片">
              <a style={{ fontSize: 12, cursor: 'pointer', color: 'var(--semi-color-primary)' }} onClick={() => set('cartPic', { picUrl: '', picThumb: '' })}>{t('重置')}</a>
            </HeaderField>
            <Input value={header.cartPic?.picUrl || ''} onChange={(v) => set('cartPic', { picUrl: v, picThumb: v })} placeholder={t('购物车图片 URL')} size="small" />
          </div>
        )}
        <HeaderField label="顶部菜单购物车">
          <RadioGroup value={Number(header.topMenuCartShow ?? 1)} onChange={(e: any) => set('topMenuCartShow', Number(e.target.value))} type="button">
            <Radio value={1}>{t('显示')}</Radio>
            <Radio value={0}>{t('隐藏')}</Radio>
          </RadioGroup>
        </HeaderField>
      </HeaderSection>

      <HeaderSection title={t('搜索条')}>
        <HeaderField label="搜索条颜色">
          <RadioGroup value={Number(header.searchColorType ?? 1)} onChange={(e: any) => set('searchColorType', Number(e.target.value))} type="button">
            <Radio value={1}>{t('系统')}</Radio>
            <Radio value={2}>{t('自定义')}</Radio>
          </RadioGroup>
        </HeaderField>
        {Number(header.searchColorType ?? 1) === 2 && (
          <HeaderField label="自定义颜色">
            <HeaderColorInput value={header.searchColor || ''} onChange={(v) => set('searchColor', v)} />
          </HeaderField>
        )}
        <HeaderField label="所有商品分类">
          <RadioGroup value={Number(header.showCategory ?? 1)} onChange={(e: any) => set('showCategory', Number(e.target.value))} type="button">
            <Radio value={1}>{t('开启')}</Radio>
            <Radio value={0}>{t('关闭')}</Radio>
          </RadioGroup>
        </HeaderField>
        <HeaderField label="首页分类">
          <RadioGroup value={Number(header.showHomeCategory ?? 1)} onChange={(e: any) => set('showHomeCategory', Number(e.target.value))} type="button">
            <Radio value={1}>{t('显示')}</Radio>
            <Radio value={0}>{t('隐藏')}</Radio>
          </RadioGroup>
        </HeaderField>
        <HeaderField label="分类上间距">
          <SliderWithInput value={Number(header.categoryMarginTop) || 0} min={0} max={60} onChange={(v) => set('categoryMarginTop', v)} />
        </HeaderField>
      </HeaderSection>

      <HeaderSection title={t('导航栏')}>
        <HeaderField label="主导航">
          <RadioGroup value={Number(header.showMainNav ?? 1)} onChange={(e: any) => set('showMainNav', Number(e.target.value))} type="button">
            <Radio value={1}>{t('显示')}</Radio>
            <Radio value={0}>{t('隐藏')}</Radio>
          </RadioGroup>
        </HeaderField>
        <HeaderField label="导航栏上间距">
          <SliderWithInput value={Number(header.mainNavPaddingTop) || 0} min={0} max={60} onChange={(v) => set('mainNavPaddingTop', v)} />
        </HeaderField>
        <HeaderField label="导航栏下间距">
          <SliderWithInput value={Number(header.mainNavPaddingBottom) || 0} min={0} max={60} onChange={(v) => set('mainNavPaddingBottom', v)} />
        </HeaderField>
      </HeaderSection>

      <HeaderSection title={t('顶部提示条')}>
        <HeaderField label="展示 benefit bar">
          <Switch checked={Number(header.showBenefitBar ?? 1) === 1} onChange={(v) => set('showBenefitBar', v ? 1 : 0)} />
        </HeaderField>
        {Number(header.showBenefitBar ?? 1) === 1 && (
          <>
            <HeaderField label="benefit bar 背景色">
              <HeaderColorInput value={header.benefitBarBackgroundColor || ''} onChange={(v) => set('benefitBarBackgroundColor', v)} />
            </HeaderField>
            {!simple && (
              <>
                <HeaderField label="benefit 主标题颜色">
                  <HeaderColorInput value={header.benefitTitleColor || ''} onChange={(v) => set('benefitTitleColor', v)} />
                </HeaderField>
                <HeaderField label="benefit 副标题颜色">
                  <HeaderColorInput value={header.benefitDescColor || ''} onChange={(v) => set('benefitDescColor', v)} />
                </HeaderField>
                <HeaderField label="顶部菜单文字颜色">
                  <HeaderColorInput value={header.topMenuTextColor || ''} onChange={(v) => set('topMenuTextColor', v)} />
                </HeaderField>
                <HeaderField label="顶部菜单文字悬浮颜色">
                  <HeaderColorInput value={header.topMenuTextHoverColor || ''} onChange={(v) => set('topMenuTextHoverColor', v)} />
                </HeaderField>
                <div className="pchead-benefit-content">
                  <div className="pchead-benefit-content__label">{t('顶部提示内容')}</div>
                  <BenefitGroupsEditor
                    groups={(header.benefitNoticeGroups || []) as Array<Record<string, any>>}
                    onChange={updateBenefitGroups}
                  />
                </div>
              </>
            )}
          </>
        )}
      </HeaderSection>

      {simple && (
        <HeaderSection title={t('顶部提示条（简约风）')}>
          <HeaderField label="顶部菜单文字颜色">
            <HeaderColorInput value={header.topMenuTextColor || ''} onChange={(v) => set('topMenuTextColor', v)} />
          </HeaderField>
          <HeaderField label="顶部菜单文字悬浮颜色">
            <HeaderColorInput value={header.topMenuTextHoverColor || ''} onChange={(v) => set('topMenuTextHoverColor', v)} />
          </HeaderField>
          <HeaderField label="站点导航">
            <RadioGroup value={Number(header.showTopSiteNav ?? 1)} onChange={(e: any) => set('showTopSiteNav', Number(e.target.value))} type="button">
              <Radio value={1}>{t('显示')}</Radio>
              <Radio value={0}>{t('隐藏')}</Radio>
            </RadioGroup>
          </HeaderField>
          <HeaderField label="会员信息">
            <RadioGroup value={Number(header.showTopMemberName ?? 1)} onChange={(e: any) => set('showTopMemberName', Number(e.target.value))} type="button">
              <Radio value={1}>{t('显示')}</Radio>
              <Radio value={0}>{t('隐藏')}</Radio>
            </RadioGroup>
          </HeaderField>
          <HeaderField label="地区选择">
            <RadioGroup value={Number(header.showTopRegion ?? 1)} onChange={(e: any) => set('showTopRegion', Number(e.target.value))} type="button">
              <Radio value={1}>{t('显示')}</Radio>
              <Radio value={0}>{t('隐藏')}</Radio>
            </RadioGroup>
          </HeaderField>
          <HeaderField label="显示右侧菜单（不含站点导航、购物车）">
            <Switch checked={Number(header.showTopMenuBar ?? 1) === 1} onChange={(v) => set('showTopMenuBar', v ? 1 : 0)} />
          </HeaderField>
          {Number(header.showTopMenuBar ?? 1) !== 0 && (
            <>
              <HeaderField label="右侧菜单">
                <RadioGroup value={Number(header.showTopUserMenu ?? 1)} onChange={(e: any) => set('showTopUserMenu', Number(e.target.value))} type="button">
                  <Radio value={1}>{t('显示')}</Radio>
                  <Radio value={0}>{t('隐藏')}</Radio>
                </RadioGroup>
              </HeaderField>
              {Number(header.showTopUserMenu ?? 1) !== 0 && (
                <>
                  <HeaderField label="站内消息">
                    <RadioGroup value={Number(header.showTopMessage ?? 1)} onChange={(e: any) => set('showTopMessage', Number(e.target.value))} type="button">
                      <Radio value={1}>{t('显示')}</Radio>
                      <Radio value={0}>{t('隐藏')}</Radio>
                    </RadioGroup>
                  </HeaderField>
                  <HeaderField label="我的订单">
                    <RadioGroup value={Number(header.showTopOrders ?? 1)} onChange={(e: any) => set('showTopOrders', Number(e.target.value))} type="button">
                      <Radio value={1}>{t('显示')}</Radio>
                      <Radio value={0}>{t('隐藏')}</Radio>
                    </RadioGroup>
                  </HeaderField>
                  <HeaderField label="会员中心">
                    <RadioGroup value={Number(header.showTopMemberCenter ?? 1)} onChange={(e: any) => set('showTopMemberCenter', Number(e.target.value))} type="button">
                      <Radio value={1}>{t('显示')}</Radio>
                      <Radio value={0}>{t('隐藏')}</Radio>
                    </RadioGroup>
                  </HeaderField>
                  {Number(header.showTopMemberCenter ?? 1) !== 0 && (
                    <div className="pchead-member-menu">
                      <Text type="tertiary" size="small">{t('会员中心菜单')}</Text>
                      {((header.memberCenterItems || []) as Array<Record<string, any>>).map((it, index) => (
                        <div className="pchead-member-menu__item" key={index}>
                          <div className="pchead-member-menu__head">
                            <Text strong size="small">{t('第【{index}】项', { index: index + 1 })}</Text>
                            <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} onClick={() => updateMemberItems((header.memberCenterItems || []).filter((_: any, i: number) => i !== index))} />
                          </div>
                          <Input
                            value={it.picTitle || ''}
                            onChange={(v) => updateMemberItems((header.memberCenterItems || []).map((x: any, i: number) => (i === index ? { ...x, picTitle: v } : x)))}
                            placeholder={t('菜单名')}
                            size="small"
                          />
                          <div className="pchead-member-menu__link">
                            <LinkField
                              value={it.picLink}
                              onChange={(link) => updateMemberItems((header.memberCenterItems || []).map((x: any, i: number) => (i === index ? { ...x, picLink: link } : x)))}
                            />
                          </div>
                        </div>
                      ))}
                      <Button size="small" icon={<IconPlus />} style={{ marginTop: 6 }} onClick={() => updateMemberItems([...(header.memberCenterItems || []), { picTitle: '', picDesc: '', picLink: {} }])}>
                        {t('添加菜单')}
                      </Button>
                    </div>
                  )}
                </>
              )}
            </>
          )}
        </HeaderSection>
      )}
    </div>
  );
}

/* ===================== 模块设置面板（字段驱动） ===================== */

function quickNavSection(field: SettingFieldDef): string | null {
  if (field.key === 'items') return null;
  if (['style.fontSize', 'style.fontWeight', 'style.lineHeight', 'style.textColor', 'style.hoverColor'].includes(field.key)) {
    return '样式设置';
  }
  if (['style.iconSize', 'style.iconGap'].includes(field.key)) return '图标设置';
  if (['style.containerAlign', 'style.itemGap', 'style.singleLine'].includes(field.key)) return '布局设置';
  return null;
}

function ModuleSettingsPanel({
  def,
  moduleData,
  onChange,
}: {
  def: ModuleDef;
  moduleData: Record<string, any>;
  onChange: (next: Record<string, any>) => void;
}) {
  const { t } = useI18n();
  const isQuickNav = def.type === 'pc-home-quick-nav';
  /**
   * 是否属于「扩展设置」Tab：
   * - 显式 extend:true；
   * - frame.* 容器字段；
   * - container 变体默认归扩展，但显式 extend:false 时归内容设置
   *   （原版 pc-product-group 的「购买按钮」位于内容设置/商品参数设置之后）。
   */
  const isExtendField = (field: ModuleDef['settings'][number]) => {
    if (field.key.startsWith('frame.')) return field.extend !== false;
    if (field.kind === 'container') return field.extend !== false;
    return Boolean(field.extend);
  };
  const hasExtend = def.settings.some(isExtendField);
  const tabbed = isQuickNav || hasExtend;
  const [activeTab, setActiveTab] = useState<'content' | 'extend'>('content');
  useEffect(() => {
    setActiveTab('content');
  }, [def.type]);
  const visibleSettings = tabbed
    ? def.settings.filter((field) => (activeTab === 'extend' ? isExtendField(field) : !isExtendField(field)))
    : def.settings;
  const blockKinds = new Set(['picList', 'pic', 'textarea', 'products', 'title', 'container', 'hotarea', 'news', 'groupon', 'squareTemplate']);
  const contentTabLabel = def.type === 'pc-product-group' ? t('商品设置') : '内容设置';
  const SECTION_LEVELS: Record<string, 1 | 2> = {
    // 模块级 / 内容级 / 扩展级
    '内容设置': 1,
    '标题设置': 1,
    '商品设置': 1,
    '选择布局': 1,
    '添加图片': 1,
    '添加导航项': 1,
    '添加轮播图片': 1,
    '添加分组Banner轮播大图': 1,
    '外边距': 1,
    '容器设置': 1,
    '商品颜色设置': 1,
    '分组标题设置': 1,
    '模块样式设置': 1,
    '内容区样式设置': 1,
    // 营销通栏一级区
    '左侧分类导航': 1,
    '中间焦点区': 1,
    '右侧会员卡': 1,
    '快捷入口区': 1,
    '整体样式': 1,
    // 二级子分组
    '轮播广告': 2,
    '推荐卡位': 2,
    '入口列表': 2,
    '商品参数设置': 2,
    '参数设置': 2,
    '样式设置': 2,
    '图标设置': 2,
    '布局设置': 2,
    '购买按钮': 2,
  };
  const sectionClass = (name: string) => (
    `dec-edit-title-box${SECTION_LEVELS[name] === 1 ? ' dec-edit-title-box--primary' : ' dec-edit-title-box--secondary'}`
  );
  const EXTEND_TITLES: Record<string, string> = {
    // pc-banner2：原版为「外边距」，新增「组件高度」后改「容器设置」（与分类导航3 同款，组件高度在标题下第一项）
    'pc-banner2': '容器设置',
    'pc-group-banner': '外边距',
    'pc-category-nav': '外边距',
    'pc-category-nav3': '容器设置',
    'pc-home-coupon': '容器设置',
    'pc-home-groupon': '容器设置',
    'pc-home-hero': '外边距',
    'pc-home-quick-nav': '容器设置',
    'pc-home-seckill': '外边距',
    'pc-image-hotarea': '容器设置',
    'pc-image-nav': '容器设置',
    'pc-image-square-ad': '容器设置',
    'pc-notice': '容器设置',
    'pc-product': '容器设置',
    'pc-recommend-product': '外边距',
    'pc-spacer': '外边距',
    'pc-title-ad': '容器设置',
  };
  const extendTitle = EXTEND_TITLES[def.type];
  const resetModuleToDefaults = () => onChange(JSON.parse(JSON.stringify(def.defaults())));
  const CONTENT_TITLES: Record<string, string> = {
    'pc-banner2': '添加轮播图片',
    'pc-group-banner': '添加分组Banner轮播大图',
    'pc-home-quick-nav': '内容设置',
    'pc-category-nav3': '内容设置',
    'pc-home-groupon': '标题设置',
    'pc-home-seckill': '标题设置',
    'pc-product-group': '内容设置',
    'pc-title-ad': '标题设置',
  };
  const contentTitle = CONTENT_TITLES[def.type];

  return (
    <DecSettingsPanel
      className="pc-module-settings"
      title={t(def.label)}
      desc={def.desc ? t(def.desc) : undefined}
      tabs={tabbed ? [
        { key: 'content', label: t(contentTabLabel) },
        { key: 'extend', label: t('扩展设置') },
      ] : undefined}
      activeTab={activeTab}
      onTabChange={(key) => setActiveTab(key as 'content' | 'extend')}
      extra={def.type === 'pc-notice' ? (
        <Popconfirm
          title={t('恢复系统默认风格？')}
          content={t('公告文字、图标与样式将重置为默认值')}
          okText={t('重置')}
          cancelText={t('取消')}
          onConfirm={resetModuleToDefaults}
        >
          <Button size="small" theme="borderless" icon={<IconRefresh />}>{t('全局重置')}</Button>
        </Popconfirm>
      ) : undefined}
    >
      <div className="pc-module-settings__body">
        {tabbed && activeTab === 'extend' && extendTitle && (
          <div className={sectionClass(extendTitle)}><div className="title">{t(extendTitle)}</div></div>
        )}
        {((tabbed && activeTab === 'content') || !tabbed) && contentTitle && (
          <div className={sectionClass(contentTitle)}><div className="title">{t(contentTitle)}</div></div>
        )}
        {def.note && <div className="dec-edit-group-desc" style={{ padding: '0 16px 8px' }}>{t(def.note)}</div>}
        {visibleSettings.length === 0 && <div className="dec-edit-group-desc" style={{ padding: '12px 16px' }}>{t('该模块无可配置项')}</div>}
        {(() => {
          let lastSection: string | null = null;
          return visibleSettings.map((field) => {
            if (field.showIf && !field.showIf(moduleData)) return null;
            let section = isQuickNav && activeTab === 'content' ? quickNavSection(field) : field.section;
            if (def.type === 'pc-image-nav' && field.key === 'picList') {
              section = Number(moduleData.navStyle) === 3 ? tl('添加导航项') : '添加图片';
            }
            const sectionList = (field.sections?.length ? field.sections : section ? [section] : [])
              .filter((item) => item !== lastSection);
            if (sectionList.length) lastSection = sectionList[sectionList.length - 1];
            const showSection = sectionList.length > 0;
            const isSection = field.kind === 'title' || field.kind === 'container';
            if (isSection) {
              return (
                <div key={field.key}>
                  {!field.hideSection && (
                    <div className={sectionClass(field.label)}><div className="title">{t(field.label)}</div></div>
                  )}
                  <FieldControl field={field} moduleData={moduleData} onChange={onChange} />
                  {/* 此前 isSection 分支不渲染 hint，导致 kind:'title' 传的提示（如秒杀「秒杀商品由营销-秒杀决定」）是死参数 */}
                  {fieldHint(field, moduleData) && <div className="dec-edit-group-desc">{t(fieldHint(field, moduleData))}</div>}
                </div>
              );
            }
            if (field.kind === 'heroCards') {
              return (
                <div key={field.key}>
                  <div className={sectionClass(field.section || field.label)}><div className="title">{t(field.section || field.label)}</div></div>
                  <FieldControl field={field} moduleData={moduleData} onChange={onChange} />
                </div>
              );
            }
            const block = blockKinds.has(field.kind);
            return (
              <div key={field.key}>
                {sectionList.map((item) => (
                  <div className={sectionClass(item)} key={item}><div className="title">{t(item)}</div></div>
                ))}
                <div className={`dec-edit-group${block ? ' dec-edit-group-block' : ''}${field.hideLabel ? ' dec-edit-group--nolabel' : ''}`}>
                  {!field.hideLabel && (
                    <div className="dec-edit-group-title">
                      <div className="label">{t(field.label)}</div>
                    </div>
                  )}
                  <div className="dec-edit-group-con">
                    <FieldControl field={field} moduleData={moduleData} onChange={onChange} />
                  </div>
                  {fieldHint(field, moduleData) && <div className="dec-edit-group-desc">{t(fieldHint(field, moduleData))}</div>}
                </div>
              </div>
            );
          });
        })()}
      </div>
    </DecSettingsPanel>
  );
}

/* ===================== 字段控件 ===================== */

let categoryCache: Array<{ label: string; value: number }> | null = null;

function useCategoryOptions() {
  const [options, setOptions] = useState<Array<{ label: string; value: number }>>(categoryCache || []);
  useEffect(() => {
    if (categoryCache) return;
    apiGet<any[]>('/product/category/getAllCategory')
      .then((tree) => {
        const flat: Array<{ label: string; value: number }> = [];
        const traverse = (nodes: any[], prefix = '') => {
          nodes.forEach((n) => {
            const label = prefix ? `${prefix}└─ ${n.categoryName}` : n.categoryName;
            flat.push({ label, value: n.categoryId });
            if (Array.isArray(n.children) && n.children.length > 0) traverse(n.children, `${prefix}    `);
          });
        };
        traverse(Array.isArray(tree) ? tree : []);
        categoryCache = flat;
        setOptions(flat);
      })
      .catch(() => setOptions([]));
  }, []);
  return options;
}

function ProductGroupPickerModal({
  visible,
  current,
  onOk,
  onCancel,
}: {
  visible: boolean;
  current?: Record<string, any>;
  onOk: (group: { productGroupId: number; productGroupName: string }) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [rows, setRows] = useState<Array<{ productGroupId: number; productGroupName: string }>>([]);
  const [loading, setLoading] = useState(false);
  const [picked, setPicked] = useState<{ productGroupId: number; productGroupName: string } | null>(
    current && current.productGroupId ? { productGroupId: Number(current.productGroupId), productGroupName: String(current.productGroupName ?? '') } : null,
  );
  const load = () => {
    setLoading(true);
    apiGet<{ records?: any[]; list?: any[] }>('/product/productGroup/list', { page: 1, size: 50, keyword: keyword || undefined })
      .then((res) => setRows((res.records || res.list || []).map((row: any) => ({ productGroupId: Number(row.productGroupId), productGroupName: String(row.productGroupName || '') }))))
      .catch(() => setRows([]))
      .finally(() => setLoading(false));
  };
  useEffect(() => {
    if (visible) {
      setPicked(
        current && current.productGroupId
          ? { productGroupId: Number(current.productGroupId), productGroupName: String(current.productGroupName ?? '') }
          : null,
      );
      load();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);
  return (
    <Modal
      title={t('选择商品分组')}
      visible={visible}
      onCancel={onCancel}
      width={560}
      onOk={() => { if (picked) onOk(picked); }}
      okText={t('确定')}
      cancelText={t('取消')}
      okButtonProps={{ disabled: !picked }}
    >
      <div className="dec-group-picker">
        <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
          <Input value={keyword} onChange={setKeyword} placeholder={t('输入分组名称')} onEnterPress={load} showClear onClear={() => { setKeyword(''); setTimeout(load, 0); }} />
          <Button onClick={load} loading={loading}>{t('搜索')}</Button>
        </div>
        <div className="dec-group-picker__list">
          {rows.map((row) => {
            const active = picked?.productGroupId === row.productGroupId;
            return (
              <div
                key={row.productGroupId}
                className={`dec-group-picker__item${active ? ' is-active' : ''}`}
                onClick={() => setPicked(row)}
              >
                <span className="dec-group-picker__name">{row.productGroupName}</span>
                <span className="dec-group-picker__id">ID {row.productGroupId}</span>
                {active && <span className="dec-group-picker__check">✓</span>}
              </div>
            );
          })}
          {!rows.length && !loading && <Text type="tertiary">{t('暂无商品分组')}</Text>}
        </div>
      </div>
    </Modal>
  );
}

/* ProductPickerModal 已抽到共享 components/ProductPickerModal.tsx（2026-11-15），顶部统一 import。 */

function GrouponPickerModal({
  visible,
  current,
  onOk,
  onCancel,
}: {
  visible: boolean;
  current?: Record<string, any> | null;
  onOk: (item: Record<string, any> | null) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [rows, setRows] = useState<Array<Record<string, any>>>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [picked, setPicked] = useState<Record<string, any> | null>(current || null);

  const rowId = (row: Record<string, any>) => row.grouponPromotionId ?? row.grouponId ?? row.id;

  const load = (nextPage = 1) => {
    setLoading(true);
    apiGet<{ records?: Array<Record<string, any>>; list?: Array<Record<string, any>>; total?: number }>('/promotion/groupon/list', { page: nextPage, size: 20 })
      .then((res) => {
        const list = res.records || res.list || [];
        setRows(list);
        setTotal(Number(res.total ?? list.length));
      })
      .catch(() => setRows([]))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    if (!visible) return;
    setPicked(current || null);
    setPage(1);
    load(1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  /** 活动时间描述：未开始/进行中/已结束 + 起止时间。 */
  const activityTime = (row: Record<string, any>): { label: string; range: string } => {
    const fmt = (v: unknown) => String(v ?? '').slice(0, 16);
    const start = fmt(row.startTime);
    const end = fmt(row.endTime);
    const ts = (v: unknown) => {
      const ms = new Date(String(v ?? '').replace(/-/g, '/')).getTime();
      return Number.isFinite(ms) ? ms : 0;
    };
    const now = Date.now();
    const s = ts(row.startTime);
    const e = ts(row.endTime);
    const label = s && now < s ? t('未开始') : e && now > e ? t('已结束') : t('进行中');
    return { label, range: start && end ? `${start} ~ ${end}` : start || end || '' };
  };

  return (
    <Modal
      title={t('选择拼团活动')}
      visible={visible}
      width={640}
      onOk={() => onOk(picked)}
      onCancel={onCancel}
      okText={t('确定')}
      cancelText={t('取消')}
      okButtonProps={{ disabled: !picked }}
    >
      <div className="dec-groupon-picker">
        <div className="dec-groupon-picker__list">
          {loading && <div className="dec-groupon-picker__loading"><Spin /></div>}
          {!loading && rows.length === 0 && <Text type="tertiary">{t('暂无拼团活动')}</Text>}
          {!loading && rows.map((row) => {
            const id = rowId(row);
            const name = row.name || row.title || t('活动#{id}', { id });
            const isPicked = picked && (picked.grouponPromotionId ?? picked.grouponId ?? picked.id) === id;
            const isLadder = Number(row.groupType) === 2;
            const time = activityTime(row);
            return (
              <div
                key={String(id)}
                className={`dec-groupon-picker__item${isPicked ? ' is-active' : ''}`}
                onClick={() => setPicked({ grouponPromotionId: id, name, groupType: row.groupType, startTime: row.startTime, endTime: row.endTime })}
              >
                <div className="dec-groupon-picker__main">
                  <div className="dec-groupon-picker__name">{name}</div>
                  <div className="dec-groupon-picker__meta">
                    <span className={`dec-groupon-picker__type${isLadder ? ' is-ladder' : ''}`}>{isLadder ? t('阶梯拼团') : t('普通拼团')}</span>
                    <span className={`dec-groupon-picker__status is-${time.label === t('进行中') ? 'on' : 'off'}`}>{time.label}</span>
                    {time.range && <span className="dec-groupon-picker__time">{time.range}</span>}
                  </div>
                </div>
                {isPicked && <span className="dec-groupon-picker__check">✓</span>}
              </div>
            );
          })}
        </div>
        {total > 20 && (
          <div className="dec-groupon-picker__pager">
            <Pagination
              currentPage={page}
              pageSize={20}
              total={total}
              onPageChange={(p) => { setPage(p); load(p); }}
              size="small"
            />
          </div>
        )}
      </div>
    </Modal>
  );
}


/** 单图字段（公告图标 / LOGO 等）：缩略图 + 素材中心直选 + 恢复默认。 */
function SinglePicField({
  value,
  onChange,
  resetValue,
  hint,
}: {
  value: unknown;
  onChange: (next: PicItem) => void;
  resetValue?: unknown;
  hint?: string;
}) {
  const { t } = useI18n();
  const [pickerOpen, setPickerOpen] = useState(false);
  const current = (value && typeof value === 'object' ? value : {}) as PicItem;
  const url = current.picUrl || current.picThumb || '';
  const fallback = (resetValue && typeof resetValue === 'object' ? resetValue : null) as PicItem | null;
  const defaultUrl = fallback?.picUrl || fallback?.picThumb || '';
  const isDefault = Boolean(defaultUrl) && url === defaultUrl;
  const applyUrls = (urls: string[]) => {
    const nextUrl = urls[0] || '';
    onChange({ ...current, picUrl: nextUrl, picThumb: nextUrl });
    setPickerOpen(false);
  };
  return (
    <div className="dec-single-pic">
      <button
        type="button"
        className="dec-single-pic__thumb"
        onClick={() => setPickerOpen(true)}
        title={t('从素材中心选择')}
      >
        {url ? (
          <>
            <img src={url} alt="" />
            <span className="dec-single-pic__mask">{t('更换')}</span>
          </>
        ) : (
          <IconPlus />
        )}
      </button>
      <div className="dec-single-pic__body">
        <div className="dec-single-pic__name">{url ? t('已选择图片') : t('未设置图片')}</div>
        <div className="dec-single-pic__hint">{hint || t('支持 JPG / PNG / WebP / SVG，建议使用透明底图标')}</div>
        <div className="dec-single-pic__actions">
          <Button size="small" onClick={() => setPickerOpen(true)}>
            {url ? t('更换图片') : t('从素材中心选择')}
          </Button>
          {url ? (
            <Button size="small" theme="borderless" type="danger" onClick={() => onChange({ picUrl: '', picThumb: '' })}>
              {t('删除')}
            </Button>
          ) : null}
          {fallback && !isDefault ? (
            <Button size="small" theme="borderless" onClick={() => onChange({ ...fallback })}>
              {t('恢复默认')}
            </Button>
          ) : null}
        </div>
      </div>
      <GalleryPickerModal
        visible={pickerOpen}
        multiple={false}
        limit={1}
        onCancel={() => setPickerOpen(false)}
        onConfirm={applyUrls}
      />
    </div>
  );
}

function PicListEditor({
  value,
  onChange,
  showDesc = false,
  addText,
  showLink = true,
  showText = true,
  max,
}: {
  value: PicItem[];
  onChange: (next: PicItem[]) => void;
  showDesc?: boolean;
  addText?: string;
  showLink?: boolean;
  showText?: boolean;
  max?: number;
}) {
  const { t } = useI18n();
  const list = Array.isArray(value) ? value : [];
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [overIndex, setOverIndex] = useState<number | null>(null);
  const [editIndex, setEditIndex] = useState<number | null>(null);
  const [productPickerOpen, setProductPickerOpen] = useState(false);
  const updateItem = (index: number, patch: Record<string, any>) => {
    onChange(list.map((it, i) => (i === index ? { ...it, ...patch } : it)));
  };
  const reorder = (from: number, to: number) => {
    if (from === to) return;
    const next = [...list];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onChange(next);
  };
  const endDrag = () => {
    setDragIndex(null);
    setOverIndex(null);
  };
  const removeAt = (index: number) => {
    onChange(list.filter((_, i) => i !== index));
    if (editIndex === index) setEditIndex(null);
    else if (editIndex !== null && editIndex > index) setEditIndex(editIndex - 1);
  };
  const addItem = () => {
    const next = [
      ...list,
      { picUrl: '', picThumb: '', picName: '', picTitle: '', picLink: { link: '', name: '' } } as PicItem,
    ];
    onChange(next);
    setEditIndex(next.length - 1);
  };
  const current = editIndex !== null ? list[editIndex] : undefined;
  const currentUrl = current ? String(current.picUrl || current.picThumb || '') : '';
  const renderEditor = (index: number) => (
    <div className="dec-pic-modal">
      <div className="dec-pic-modal__toolbar">
        <Button size="small" icon={<IconPlus />} onClick={() => setProductPickerOpen(true)}>
          {t('从商品选择图片')}
        </Button>
        <span className="dec-pic-modal__toolbar-hint">{t('可用商品头图或商品相册图片')}</span>
        {currentUrl ? (
          <Button
            size="small"
            theme="borderless"
            type="danger"
            onClick={() => updateItem(index, { picUrl: '', picThumb: '' })}
          >
            {t('清除图片')}
          </Button>
        ) : null}
      </div>
      <div className="dec-pic-modal__preview">
        <ImageUploader
          single
          value={currentUrl ? [currentUrl] : []}
          onChange={(urls) => updateItem(index, { picUrl: urls[0] || '', picThumb: urls[0] || '' })}
        />
        <div className="dec-pic-modal__preview-hint">{t('点击图片从素材中心更换')}</div>
      </div>
      {showText && (
        <label className="dec-pic-modal__field">
          <span>{t('标题')}</span>
          <Input
            value={current?.picTitle || ''}
            placeholder={t('建议10个字以内，可不填')}
            onChange={(v) => updateItem(index, { picTitle: v })}
          />
        </label>
      )}
      {showDesc && (
        <label className="dec-pic-modal__field">
          <span>{t('描述')}</span>
          <Input
            value={current?.picDesc || ''}
            placeholder={t('可不填')}
            onChange={(v) => updateItem(index, { picDesc: v })}
          />
        </label>
      )}
      {showLink && (
        <div className="dec-pic-modal__field">
          <span>{t('链接')}</span>
          <LinkField
            value={current?.picLink}
            onChange={(link) => updateItem(index, { picLink: link })}
          />
        </div>
      )}
    </div>
  );
  return (
    <div className="dec-pic-grid">
      {list.map((item, index) => {
        const url = String(item.picUrl || item.picThumb || '');
        return (
          <div
            className={`dec-pic-grid__card${dragIndex === index ? ' is-dragging' : ''}${
              overIndex === index && dragIndex !== null && dragIndex !== index ? ' is-drop-target' : ''
            }`}
            key={String(item.picId || index)}
            role="button"
            tabIndex={0}
            onClick={() => setEditIndex(index)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') setEditIndex(index);
            }}
            onDragOver={(event) => {
              if (dragIndex === null) return;
              event.preventDefault();
              if (overIndex !== index) setOverIndex(index);
            }}
            onDrop={(event) => {
              event.preventDefault();
              if (dragIndex !== null) reorder(dragIndex, index);
              endDrag();
            }}
          >
            <div className="dec-pic-grid__thumb">
              {url ? <img src={url} alt="" /> : <div className="dec-pic-grid__empty"><IconPlus /></div>}
              <span className="dec-pic-grid__index">{index + 1}</span>
              <span
                className="dec-pic-grid__handle"
                draggable
                title={t('拖拽排序')}
                onClick={(event) => event.stopPropagation()}
                onDragStart={(event) => {
                  event.stopPropagation();
                  setDragIndex(index);
                  event.dataTransfer.effectAllowed = 'move';
                  event.dataTransfer.setData('text/plain', String(index));
                }}
                onDragEnd={endDrag}
              >
                <IconHandle />
              </span>
            </div>
            <div className="dec-pic-grid__meta">
              <div className="dec-pic-grid__title">{item.picTitle || t('未命名')}</div>
              {showDesc && item.picDesc ? <div className="dec-pic-grid__desc">{item.picDesc}</div> : null}
            </div>
          </div>
        );
      })}
      {(!max || list.length < max) && (
        <button type="button" className="dec-pic-grid__add" onClick={addItem}>
          <span className="dec-pic-grid__add-icon"><IconPlus /></span>
          <span>{t(addText || '添加图片')}</span>
        </button>
      )}
      <Modal
        title={t('编辑图片')}
        visible={editIndex !== null}
        onCancel={() => setEditIndex(null)}
        width={560}
        footer={
          <div className="dec-pic-modal__footer">
            <Button
              type="danger"
              theme="borderless"
              icon={<IconClose />}
              onClick={() => editIndex !== null && removeAt(editIndex)}
            >
              {t('删除')}
            </Button>
            <Button theme="solid" type="primary" onClick={() => setEditIndex(null)}>
              {t('完成')}
            </Button>
          </div>
        }
      >
        {editIndex !== null ? renderEditor(editIndex) : null}
      </Modal>
      <ProductImagePickerModal
        visible={productPickerOpen}
        onCancel={() => setProductPickerOpen(false)}
        onPick={(url) => {
          if (editIndex !== null) updateItem(editIndex, { picUrl: url, picThumb: url });
          setProductPickerOpen(false);
        }}
        onPickMany={(items) => {
          if (editIndex === null || !items.length) return;
          const currentItem = list[editIndex];
          const hasPic = !!String(currentItem?.picUrl || currentItem?.picThumb || '');
          const capacity = max ? Math.max(0, max - list.length) : Number.POSITIVE_INFINITY;
          // 当前卡片为空时第一张直接填它，不占用新增名额
          const allowance = hasPic ? capacity : capacity + 1;
          const usable = items.slice(0, allowance);
          if (items.length > usable.length) Toast.warning(t('最多只能添加 {max} 张图片', { max: max ?? 0 }));
          if (!usable.length) {
            setProductPickerOpen(false);
            return;
          }
          // 商品名做标题，链接指向 PC 前台商品详情页 /product/{id}
          const makeItem = (p: PickedImage): PicItem => ({
            picUrl: p.url,
            picThumb: p.url,
            picName: p.productName,
            picTitle: p.productName,
            picDesc: '',
            picLink: {
              link: `/product/${p.productId}`,
              name: p.productName,
              label: t('商品详情页'),
            },
          } as PicItem);
          const next = [...list];
          if (!hasPic) {
            next[editIndex] = { ...currentItem, ...makeItem(usable[0]) };
            next.splice(editIndex + 1, 0, ...usable.slice(1).map(makeItem));
          } else {
            next.splice(editIndex + 1, 0, ...usable.map(makeItem));
          }
          onChange(next);
          Toast.success(t('已带入 {count} 张商品图片（含标题和链接）', { count: usable.length }));
          setProductPickerOpen(false);
        }}
      />
    </div>
  );
}

/** 命名容器（非 frame）的字段集：pc-product-group 的扩展设置（对齐原版 components/*Settings.vue） */
const CONTAINER_VARIANTS: Record<string, SettingFieldDef[]> = {
  // 原 buyBtn 变体（对象型 showBtn/btnType 三样式）已随购买样式全站统一移除：
  // 商品分组改用模块级 buyBtnFields() 统一字段，存量对象数据由 normalizeLegacyBuyBtn 归一化。
  // 对齐原版 Settings.vue 扩展设置"商品颜色设置"
  productColor: [
    { kind: 'slider', key: 'infoLeftRightPadding', label: '商品间距', max: 20 },
    { kind: 'color', key: 'backgroundColor', label: '商品背景颜色' },
    { kind: 'color', key: 'titleColor', label: '商品标题颜色' },
    { kind: 'color', key: 'priceColor', label: '商品价格颜色' },
    { kind: 'color', key: 'crossedOutPriceColor', label: '商品划线价颜色', resetValue: '#999999' },
  ],
  // 对齐原版 Settings.vue 扩展设置"分组标题设置"（标题分割线颜色不受"分组标题"开关控制）
  groupTitle: [
    { kind: 'slider', key: 'marginLeftRight', label: '分组标题间距', max: 50 },
    { kind: 'radio', key: 'showTitle', label: '分组标题', options: [{ label: '显示', value: 1 }, { label: '不显示', value: 0 }] },
    // 与其它模块的「标题字号」一致：滑块 + 数字输入，可自定义 px，不再只有小/中/大三档
    { kind: 'slider', key: 'bigTitleFontSize', label: '标题大小', min: 12, max: 36, defaultValue: 14, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'color', key: 'titleColor', label: '标题默认颜色', showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'color', key: 'activeTitleColor', label: '标题选中颜色', showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'color', key: 'activeTitleBgColor', label: '标题选中背景色', showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'slider', key: 'activeBgMarginLeft', label: '标题选中背景色左边距', max: 60, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'slider', key: 'activeBgMarginRight', label: '标题选中背景色右边距', max: 60, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'slider', key: 'titlePaddingTopBottom', label: '标题上下内边距', max: 30, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'slider', key: 'titlePaddingLeftRight', label: '标题左右内边距', max: 30, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'slider', key: 'titleRadius', label: '标题圆角', max: 20, showIf: (m) => Number(m.showTitle) === 1 },
    { kind: 'color', key: 'splitLineColor', label: '标题分割线颜色', resetValue: 'transparent' },
  ],
  // 对齐原版 ModuleStyleSettings.vue
  moduleStyle: [
    { kind: 'slider', key: 'boxPaddingTop', label: '模块顶部间距', max: 30 },
    { kind: 'slider', key: 'boxPaddingBottom', label: '模块底部间距', max: 30 },
    { kind: 'slider', key: 'boxPadding', label: '模块左右间距', max: 30 },
    { kind: 'color', key: 'backgroundColor', label: '模块背景颜色' },
  ],
  // 对齐原版 ContentStyleSettings.vue：精简为圆角 + 背景颜色
  contentStyle: [
    { kind: 'slider', key: 'boxRadius', label: '内容区圆角半径', max: 40 },
    { kind: 'color', key: 'backgroundColor', label: '内容区背景颜色', resetValue: '#ffffff' },
  ],
};

function SettingsFieldRows({
  fields,
  data,
  onChange,
}: {
  fields: SettingFieldDef[];
  data: Record<string, any>;
  onChange: (next: Record<string, any>) => void;
}) {
  const { t } = useI18n();
  const blockKinds = new Set(['picList', 'pic', 'textarea', 'products', 'title', 'container', 'hotarea', 'news', 'groupon', 'squareTemplate', 'groupList']);
  return (
    <>
      {fields.map((field) => {
        if (field.showIf && !field.showIf(data)) return null;
        const block = blockKinds.has(field.kind);
        return (
          <div className={`dec-edit-group${block ? ' dec-edit-group-block' : ''}${field.hideLabel ? ' dec-edit-group--nolabel' : ''}`} key={field.key}>
            {!field.hideLabel && (
              <div className="dec-edit-group-title">
                <div className="label">{t(field.label)}</div>
              </div>
            )}
            <div className="dec-edit-group-con">
              <FieldControl
                field={{ ...field, key: field.key }}
                moduleData={data}
                onChange={(next) => onChange(next)}
              />
            </div>
            {fieldHint(field, data) && <div className="dec-edit-group-desc">{t(fieldHint(field, data))}</div>}
          </div>
        );
      })}
    </>
  );
}

/** 热区基准：热区坐标一律按 500px 宽的虚拟画布存储，渲染时按实际宽度等比缩放。 */
const HOTAREA_BASE = 500;
/** 新建热区的默认尺寸（基准坐标）。 */
const HOTAREA_DEFAULT_SIZE = 100;
/** 新建/拖拽时允许的最小尺寸（基准坐标）。 */
const HOTAREA_MIN_SIZE = 20;

type HotareaRect = { width?: number; height?: number; top?: number; left?: number; link?: Record<string, any> };

const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), Math.max(min, max));

/**
 * 可视化热区绘制器（现代化重写，替代原先只能手填数字的表格）。
 *
 * - 在图片上直接**拖拽框选**新建热区；
 * - 拖动热区内部移动位置，拖动四角控制点缩放；
 * - 点击热区选中，下方出现浮动的属性条（链接 / 尺寸 / 删除），也可右键打开操作菜单；
 * - 内部坐标按 500px 基准换算，与 C 端 `width/height/top/left` 契约保持一致。
 */
function HotareaPainter({
  picList,
  onChange,
}: {
  picList: Array<PicItem & { hotarea?: HotareaRect[] }>;
  onChange: (next: Array<PicItem & { hotarea?: HotareaRect[] }>) => void;
}) {
  const { t } = useI18n();
  const [activePic, setActivePic] = useState(0);
  const [selected, setSelected] = useState<number | null>(null);
  const [drag, setDrag] = useState<{ kind: 'create' | 'move' | 'resize'; corner?: string; startX: number; startY: number; origin: HotareaRect } | null>(null);
  const [menu, setMenu] = useState<{ index: number; x: number; y: number } | null>(null);
  const [linkTarget, setLinkTarget] = useState<{ index: number; link: Record<string, any> } | null>(null);
  const boardRef = useRef<HTMLDivElement | null>(null);

  const pic = picList[activePic];
  const hotareas: HotareaRect[] = (pic?.hotarea || []) as HotareaRect[];
  const picUrl = pic ? String(pic.picUrl || pic.picThumb || '') : '';
  /**
   * 画板实测尺寸。热区契约是「500px 宽基准画布」：纵轴与横轴共用同一刻度
   * （前台 PC / 编辑器画布预览 / 模块预览三处都按 容器宽度/500 缩放）。
   * 因此绘制器纵轴的换算与渲染都必须以画板**宽度**为参照，不能按画板高度归一化——
   * 否则非正方形图片上画出的热区与存储参数、前台渲染全部错位。
   */
  const [boardSize, setBoardSize] = useState({ w: 0, h: 0 });
  useEffect(() => {
    const el = boardRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) setBoardSize({ w: entry.contentRect.width, h: entry.contentRect.height });
    });
    observer.observe(el);
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activePic, picUrl, picList.length]);
  /** 纵轴在基准坐标下的上限 = 500 × 画板高宽比（宽图 < 500，竖图 > 500）。 */
  const baseHeight = boardSize.w ? (boardSize.h / boardSize.w) * HOTAREA_BASE : HOTAREA_BASE;

  const updatePic = (patch: Partial<{ hotarea: HotareaRect[] }>) => {
    onChange(picList.map((item, i) => (i === activePic ? { ...item, ...patch } : item)));
  };
  /**
   * 热区写入统一走「按最新 picList 计算」的函数式更新：
   * pointermove 高频触发时若直接闭包 `hotareas`，会读写到过期数组，导致拖拽位移被覆盖（表现为"拖不动"）。
   */
  const mutatePic = (fn: (list: Array<PicItem & { hotarea?: HotareaRect[] }>, index: number) => Array<PicItem & { hotarea?: HotareaRect[] }>) => {
    onChange(fn(picList, activePic));
  };
  const updateHotarea = (index: number, patch: Partial<HotareaRect>) => {
    mutatePic((list, pi) =>
      list.map((item, i) => {
        if (i !== pi) return item;
        const hs = (item.hotarea || []) as HotareaRect[];
        return { ...item, hotarea: hs.map((h, j) => (j === index ? { ...h, ...patch } : h)) };
      }),
    );
  };
  const removeHotarea = (index: number) => {
    updatePic({ hotarea: hotareas.filter((_, i) => i !== index) });
    setSelected(null);
    setMenu(null);
  };

  // 拖拽会话：统一在 window 上监听，鼠标移出画板也不断开
  // 所有坐标换算都实时读 boardRef 的 rect（不用 state），避免与 pointer 事件基准不一致
  useEffect(() => {
    if (!drag) return;
    const toBase = (clientX: number, clientY: number) => {
      const rect = boardRef.current?.getBoundingClientRect();
      if (!rect || !rect.width || !rect.height) return { x: 0, y: 0 };
      return {
        x: ((clientX - rect.left) / rect.width) * HOTAREA_BASE,
        // 纵轴同样按宽度归一（500px 宽基准画布），与存储契约/前台渲染一致
        y: ((clientY - rect.top) / rect.width) * HOTAREA_BASE,
      };
    };
    /** 纵轴上限（基准坐标），每次事件实时取画板矩形，避免用 state 的过期值。 */
    const maxYBase = () => {
      const rect = boardRef.current?.getBoundingClientRect();
      return rect && rect.width ? (rect.height / rect.width) * HOTAREA_BASE : HOTAREA_BASE;
    };
    const onMove = (ev: PointerEvent) => {
      const cur = toBase(ev.clientX, ev.clientY);
      const maxY = maxYBase();
      const o = drag.origin;
      if (drag.kind === 'create') {
        const x1 = clamp(Math.round(drag.startX), 0, HOTAREA_BASE);
        const y1 = clamp(Math.round(drag.startY), 0, maxY);
        const x2 = clamp(Math.round(cur.x), 0, HOTAREA_BASE);
        const y2 = clamp(Math.round(cur.y), 0, maxY);
        const next: HotareaRect = {
          left: Math.min(x1, x2),
          top: Math.min(y1, y2),
          width: Math.max(HOTAREA_MIN_SIZE, Math.abs(x2 - x1)),
          height: Math.max(HOTAREA_MIN_SIZE, Math.abs(y2 - y1)),
        };
        setDrag((d) => (d ? { ...d, origin: { ...d.origin, ...next } } : d));
        return;
      }
      const dx = cur.x - drag.startX;
      const dy = cur.y - drag.startY;
      if (drag.kind === 'move') {
        updateHotarea(selected ?? 0, {
          left: clamp(Math.round((o.left || 0) + dx), 0, HOTAREA_BASE - (o.width || 0)),
          top: clamp(Math.round((o.top || 0) + dy), 0, Math.max(0, maxY - (o.height || 0))),
        });
        return;
      }
      // resize：以起始矩形为基准，按拖动的角调整对应边
      let left = o.left || 0;
      let top = o.top || 0;
      let width = o.width || 0;
      let height = o.height || 0;
      const corner = drag.corner || 'se';
      if (corner.includes('w')) {
        const nl = clamp(Math.round(left + dx), 0, left + width - HOTAREA_MIN_SIZE);
        width = width + (left - nl);
        left = nl;
      }
      if (corner.includes('n')) {
        const nt = clamp(Math.round(top + dy), 0, top + height - HOTAREA_MIN_SIZE);
        height = height + (top - nt);
        top = nt;
      }
      if (corner.includes('e')) width = clamp(Math.round(width + dx), HOTAREA_MIN_SIZE, HOTAREA_BASE - left);
      if (corner.includes('s')) height = clamp(Math.round(height + dy), HOTAREA_MIN_SIZE, Math.max(HOTAREA_MIN_SIZE, maxY - top));
      updateHotarea(selected ?? 0, { left, top, width, height });
    };
    const onUp = () => {
      if (drag.kind === 'create') {
        const o = drag.origin;
        const w = o.width || 0;
        const h = o.height || 0;
        // 单击未拖动：落一个默认大小的热区，避免"点了没反应"
        const nextRect: HotareaRect =
          w < HOTAREA_MIN_SIZE + 2 || h < HOTAREA_MIN_SIZE + 2
            ? {
                left: clamp(Math.round(drag.startX), 0, HOTAREA_BASE - HOTAREA_DEFAULT_SIZE),
                top: clamp(Math.round(drag.startY), 0, Math.max(0, maxYBase() - HOTAREA_DEFAULT_SIZE)),
                width: HOTAREA_DEFAULT_SIZE,
                height: HOTAREA_DEFAULT_SIZE,
              }
            : { ...o };
        let addedIndex = 0;
        mutatePic((list, pi) => {
          addedIndex = ((list[pi]?.hotarea || []) as HotareaRect[]).length;
          return list.map((item, i) =>
            i === pi
              ? { ...item, hotarea: [...(((item.hotarea || []) as HotareaRect[])), { ...nextRect, link: { link: '', name: '' } }] }
              : item,
          );
        });
        setSelected(addedIndex);
      }
      setDrag(null);
    };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    return () => {
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [drag, selected, hotareas, activePic]);

  if (!picList.length) {
    return <Text type="tertiary" size="small">{t('请先在上方添加热图')}</Text>;
  }

  const pct = (v?: number) => `${((Number(v) || 0) / HOTAREA_BASE) * 100}%`;
  /** 纵向渲染值：基准坐标 × 画板宽度/500（与横轴同刻度），不能按画板高度取百分比。 */
  const vy = (v?: number) =>
    boardSize.w ? `${((((Number(v) || 0) / HOTAREA_BASE) * boardSize.w)).toFixed(2)}px` : pct(v);
  const sel = selected !== null ? hotareas[selected] : null;

  return (
    <div className="hotarea-painter">
      {picList.length > 1 && (
        <div className="hotarea-painter__tabs">
          {picList.map((item, i) => (
            <button
              key={i}
              type="button"
              className={`hotarea-painter__tab${i === activePic ? ' is-active' : ''}`}
              onClick={() => { setActivePic(i); setSelected(null); setMenu(null); }}
            >
              {t('图 {index}', { index: i + 1 })}
              <em>{(item.hotarea || []).length}</em>
            </button>
          ))}
        </div>
      )}

      <div
        className={`hotarea-painter__board${drag?.kind === 'create' ? ' is-drawing' : ''}`}
        ref={boardRef}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          if ((e.target as HTMLElement).closest('.hotarea-painter__box')) return;
          setMenu(null);
          setSelected(null);
          const rect = boardRef.current?.getBoundingClientRect();
          if (!rect || !rect.width) return;
          const startX = ((e.clientX - rect.left) / rect.width) * HOTAREA_BASE;
          const startY = ((e.clientY - rect.top) / rect.width) * HOTAREA_BASE;
          setDrag({ kind: 'create', startX, startY, origin: {} });
        }}
        onPointerLeave={() => setMenu(null)}
      >
        {picUrl ? (
          <img src={picUrl} alt="" className="hotarea-painter__img" draggable={false} />
        ) : (
          <div className="hotarea-painter__placeholder">{t('该图暂无图片，请先在「热图」中上传')}</div>
        )}

        {hotareas.map((h, i) => {
          const rect = h;
          const isSel = selected === i;
          return (
            <div
              key={i}
              className={`hotarea-painter__box${isSel ? ' is-selected' : ''}`}
              style={{ left: pct(rect.left), top: vy(rect.top), width: pct(rect.width), height: vy(rect.height) }}
              onPointerDown={(e) => {
                if ((e.target as HTMLElement).dataset.corner) return;
                e.stopPropagation();
                if (e.button !== 0) return;
                setSelected(i);
                setMenu(null);
                const host = boardRef.current?.getBoundingClientRect();
                if (!host || !host.width || !host.height) return;
                setDrag({
                  kind: 'move',
                  startX: ((e.clientX - host.left) / host.width) * HOTAREA_BASE,
                  startY: ((e.clientY - host.top) / host.width) * HOTAREA_BASE,
                  origin: { ...rect },
                });
              }}
              onClick={(e) => e.stopPropagation()}
              onContextMenu={(e) => {
                e.preventDefault();
                e.stopPropagation();
                setSelected(i);
                const host = boardRef.current?.getBoundingClientRect();
                setMenu({ index: i, x: e.clientX - (host?.left || 0), y: e.clientY - (host?.top || 0) });
              }}
            >
              <span className="hotarea-painter__label">{rect.link?.name || rect.link?.label || t('热区 {index}', { index: i + 1 })}</span>
              {isSel &&
                ['nw', 'ne', 'sw', 'se'].map((corner) => (
                  <i
                    key={corner}
                    data-corner={corner}
                    className={`hotarea-painter__handle hotarea-painter__handle--${corner}`}
                    onPointerDown={(e) => {
                      e.stopPropagation();
                      const host = boardRef.current?.getBoundingClientRect();
                      if (!host) return;
                      setDrag({
                        kind: 'resize',
                        corner,
                        startX: ((e.clientX - host.left) / host.width) * HOTAREA_BASE,
                        startY: ((e.clientY - host.top) / host.width) * HOTAREA_BASE,
                        origin: { ...rect },
                      });
                    }}
                  />
                ))}
            </div>
          );
        })}

        {drag?.kind === 'create' && (
          <div
            className="hotarea-painter__box is-draft"
            style={{ left: pct(drag.origin.left), top: vy(drag.origin.top), width: pct(drag.origin.width), height: vy(drag.origin.height) }}
          />
        )}

        {!hotareas.length && !drag && (
          <div className="hotarea-painter__hint">{t('在图片上按住并拖动即可绘制热区')}</div>
        )}

        {menu && (
          <div className="hotarea-painter__menu" style={{ left: menu.x, top: menu.y }} onPointerDown={(e) => e.stopPropagation()}>
            <button type="button" onClick={() => { setLinkTarget({ index: menu.index, link: hotareas[menu.index]?.link || {} }); setMenu(null); }}>
              {t('编辑链接')}
            </button>
            <button type="button" className="is-danger" onClick={() => removeHotarea(menu.index)}>
              {t('删除热区')}
            </button>
          </div>
        )}
      </div>

      <div className="hotarea-painter__toolbar">
        <span className="hotarea-painter__count">{t('共 {n} 个热区', { n: hotareas.length })}</span>
        <Button
          size="small"
          type="primary"
          theme="light"
          icon={<IconRefresh />}
          onClick={() => {
            updatePic({
              hotarea: [
                ...hotareas,
                { left: 20, top: 20 + hotareas.length * 20, width: HOTAREA_DEFAULT_SIZE, height: HOTAREA_DEFAULT_SIZE, link: { link: '', name: '' } },
              ],
            });
            setSelected(hotareas.length);
          }}
        >
          {t('添加热区')}
        </Button>
        {hotareas.length > 0 && (
          <Button size="small" theme="borderless" type="danger" onClick={() => { updatePic({ hotarea: [] }); setSelected(null); }}>
            {t('清空热区')}
          </Button>
        )}
      </div>

      {sel && selected !== null && (
        <div className="hotarea-painter__inspector">
          <div className="hotarea-painter__inspector-head">
            <span>{t('热区 {index}', { index: selected + 1 })}</span>
            <Button size="small" theme="borderless" type="danger" icon={<IconDelete />} onClick={() => removeHotarea(selected)} />
          </div>
          <div className="hotarea-painter__fields">
            <label>
              <span>{t('宽')}</span>
              <InputNumber min={HOTAREA_MIN_SIZE} max={HOTAREA_BASE} value={sel.width} onChange={(v) => updateHotarea(selected, { width: Number(v) || HOTAREA_MIN_SIZE })} />
            </label>
            <label>
              <span>{t('高')}</span>
              <InputNumber min={HOTAREA_MIN_SIZE} max={HOTAREA_BASE} value={sel.height} onChange={(v) => updateHotarea(selected, { height: Number(v) || HOTAREA_MIN_SIZE })} />
            </label>
            <label>
              <span>{t('左')}</span>
              <InputNumber min={0} max={HOTAREA_BASE} value={sel.left} onChange={(v) => updateHotarea(selected, { left: Number(v) || 0 })} />
            </label>
            <label>
              <span>{t('上')}</span>
              <InputNumber min={0} max={Math.round(baseHeight)} value={sel.top} onChange={(v) => updateHotarea(selected, { top: Number(v) || 0 })} />
            </label>
          </div>
          <div className="hotarea-painter__link">
            <span>{t('跳转链接')}</span>
            <LinkField value={sel.link} onChange={(link) => updateHotarea(selected, { link })} />
          </div>
        </div>
      )}

      <LinkPickerModal
        visible={!!linkTarget}
        value={linkTarget?.link}
        onCancel={() => setLinkTarget(null)}
        onOk={(link) => {
          if (linkTarget) updateHotarea(linkTarget.index, { link });
          setLinkTarget(null);
        }}
      />
    </div>
  );
}

/* ===================== 商品分类选择（右抽屉） ===================== */

/**
 * 分类原始树（GET /product/category/getAllCategory 的树形数据）。
 * 旧的 useCategoryOptions 把这棵树拍平成带「└─」缩进前缀的下拉选项，
 * 层级深时既难找也难看；抽屉选择器直接消费原始树，保留真实层级。
 */
interface CategoryTreeNode {
  categoryId: number;
  categoryName: string;
  children?: CategoryTreeNode[];
}

let categoryTreeCache: CategoryTreeNode[] | null = null;

function useCategoryTree() {
  const [tree, setTree] = useState<CategoryTreeNode[]>(categoryTreeCache || []);
  useEffect(() => {
    if (categoryTreeCache) return;
    apiGet<any[]>('/product/category/getAllCategory')
      .then((raw) => {
        const normalize = (nodes: any[]): CategoryTreeNode[] =>
          (Array.isArray(nodes) ? nodes : [])
            .map((n) => ({
              categoryId: Number(n.categoryId),
              categoryName: String(n.categoryName || ''),
              children:
                Array.isArray(n.children) && n.children.length > 0 ? normalize(n.children) : undefined,
            }))
            .filter((n) => n.categoryId > 0);
        categoryTreeCache = normalize(Array.isArray(raw) ? raw : []);
        setTree(categoryTreeCache);
      })
      .catch(() => setTree([]));
  }, []);
  return tree;
}

/** 在树中查分类的完整路径（名称数组，根在前）；找不到返回 null */
function findCategoryPath(tree: CategoryTreeNode[], id: number, trail: string[] = []): string[] | null {
  for (const node of tree) {
    const path = [...trail, node.categoryName];
    if (node.categoryId === id) return path;
    if (node.children) {
      const found = findCategoryPath(node.children, id, path);
      if (found) return found;
    }
  }
  return null;
}

function flattenCategoryTree(
  tree: CategoryTreeNode[],
  trail: string[] = [],
): Array<{ id: number; name: string; path: string }> {
  const out: Array<{ id: number; name: string; path: string }> = [];
  tree.forEach((node) => {
    const names = [...trail, node.categoryName];
    out.push({ id: node.categoryId, name: node.categoryName, path: names.join(' / ') });
    if (node.children) out.push(...flattenCategoryTree(node.children, names));
  });
  return out;
}

/**
 * 商品分类选择抽屉（右侧滑出）—— 替代「按分类」模式下拍平的 Select 下拉。
 * - 浏览模式：按真实树层级展开/收起，选中行高亮；
 * - 搜索模式：按名称过滤，候选直接显示完整路径，避免同名子分类分不清；
 * - 选择先落在抽屉内部状态，点「确定」才写回模块数据，取消不改数据；
 * - 数据契约不变：仍是单个 productCategoryId。
 */
function CategoryPickerDrawer({
  visible,
  currentId,
  tree,
  onOk,
  onCancel,
}: {
  visible: boolean;
  currentId: number;
  tree: CategoryTreeNode[];
  onOk: (id: number) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [keyword, setKeyword] = useState('');
  const [pickedId, setPickedId] = useState<number>(currentId || 0);
  const [expanded, setExpanded] = useState<Set<number>>(new Set());

  useEffect(() => {
    if (!visible) return;
    setKeyword('');
    setPickedId(currentId || 0);
    // 默认展开一级分类，并展开当前已选分类的全部祖先，保证打开即能看到已选项
    const next = new Set<number>();
    tree.forEach((n) => {
      if (n.children?.length) next.add(n.categoryId);
    });
    if (currentId) {
      const collectAncestors = (nodes: CategoryTreeNode[], trail: number[]): boolean => {
        for (const node of nodes) {
          if (node.categoryId === currentId) {
            trail.forEach((id) => next.add(id));
            return true;
          }
          if (node.children && collectAncestors(node.children, [...trail, node.categoryId])) return true;
        }
        return false;
      };
      collectAncestors(tree, []);
    }
    setExpanded(next);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  const pickedPath = pickedId ? findCategoryPath(tree, pickedId)?.join(' / ') : '';
  const searching = keyword.trim().length > 0;
  const searchResults = searching
    ? flattenCategoryTree(tree).filter((item) => item.name.includes(keyword.trim()))
    : [];

  const toggleExpand = (id: number) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const renderNodes = (nodes: CategoryTreeNode[], depth: number): ReactNode => (
    <>
      {nodes.map((node) => {
        const hasChildren = !!node.children?.length;
        const isOpen = expanded.has(node.categoryId);
        const isPicked = pickedId === node.categoryId;
        return (
          <div key={node.categoryId}>
            <div
              className={`dec-cat-row${isPicked ? ' is-picked' : ''}`}
              style={{ paddingLeft: 10 + depth * 20 }}
              onClick={() => setPickedId(node.categoryId)}
              role="button"
              aria-pressed={isPicked}
            >
              {hasChildren ? (
                <button
                  type="button"
                  className={`dec-cat-row__toggle${isOpen ? ' is-open' : ''}`}
                  aria-label={isOpen ? t('收起') : t('展开')}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleExpand(node.categoryId);
                  }}
                >
                  <IconChevronDown size="small" />
                </button>
              ) : (
                <span className="dec-cat-row__toggle-placeholder" />
              )}
              <span className="dec-cat-row__name">{node.categoryName}</span>
              {isPicked && <span className="dec-cat-row__check">{t('已选')}</span>}
            </div>
            {hasChildren && isOpen && renderNodes(node.children!, depth + 1)}
          </div>
        );
      })}
    </>
  );

  return (
    <SideSheet
      title={t('选择商品分类')}
      visible={visible}
      onCancel={onCancel}
      placement="right"
      width={560}
      className="dec-cat-sheet"
      footer={
        <div className="dec-cat-sheet__footer">
          <span className="dec-cat-sheet__current" title={pickedPath || undefined}>
            {pickedPath || t('未选择分类')}
          </span>
          {pickedId > 0 && (
            <Button theme="borderless" type="tertiary" onClick={() => setPickedId(0)}>
              {t('清除')}
            </Button>
          )}
          <Button onClick={onCancel}>{t('取消')}</Button>
          <Button theme="solid" type="primary" onClick={() => onOk(pickedId)}>
            {t('确定')}
          </Button>
        </div>
      }
    >
      <div className="dec-cat-picker">
        <Input
          value={keyword}
          onChange={setKeyword}
          placeholder={t('搜索分类名称')}
          showClear
          className="dec-cat-picker__search"
        />
        {tree.length === 0 ? (
          <div className="dec-cat-picker__empty">
            <Spin />
          </div>
        ) : searching ? (
          <div className="dec-cat-picker__list">
            {searchResults.length === 0 ? (
              <div className="dec-cat-picker__empty">{t('没有匹配的分类')}</div>
            ) : (
              searchResults.map((item) => (
                <div
                  key={item.id}
                  className={`dec-cat-row${pickedId === item.id ? ' is-picked' : ''}`}
                  onClick={() => setPickedId(item.id)}
                  role="button"
                  aria-pressed={pickedId === item.id}
                >
                  <span className="dec-cat-row__name">{item.path}</span>
                  {pickedId === item.id && <span className="dec-cat-row__check">{t('已选')}</span>}
                </div>
              ))
            )}
          </div>
        ) : (
          <div className="dec-cat-picker__list">{renderNodes(tree, 0)}</div>
        )}
      </div>
    </SideSheet>
  );
}

/**
 * 商品设置（选品）字段 —— 现代化重写（2026-11-12）。
 *
 * 旧实现是「一行 RadioGroup + 一个裸按钮 + 纯 id Tag」，选了什么商品完全不可见。
 * 现在的数据契约不变（`productIds` 是唯一事实来源，顺序即展示顺序；
 * `productItems` 只是编辑器本地的展示信息缓存，前台不读）：
 *
 * - 选品方式改为分段按钮（Semi RadioGroup type='button'）；
 * - 「选商品」复用通用组件 ProductPickGrid（缩略图卡片 / 拖拽排序 / 移除 /
 *   虚线添加卡 / 空态 / 已选计数 / 历史数据自动补全），与「商品分组」里的选品控件同源；
 * - 「按分类 / 按标签」为带标签的整行控件 + 商品数量步进器。
 */
function ProductsSettingField({
  value,
  onChange,
  field,
  categoryOptions,
}: {
  value: Record<string, any>;
  onChange: (next: Record<string, any>) => void;
  field: SettingFieldDef;
  categoryOptions: Array<{ value: string | number; label: string }>;
}) {
  const { t } = useI18n();
  const [pickerVisible, setPickerVisible] = useState(false);
  const [catPickerVisible, setCatPickerVisible] = useState(false);
  const categoryTree = useCategoryTree();
  const ids = (Array.isArray(value.productIds) ? value.productIds : []) as number[];
  const items = (Array.isArray(value.productItems) ? value.productItems : []) as ProductPickItem[];
  const selectType = Number(value.productSelectType ?? 1);

  // 卡片渲染、拖拽排序、移除、空态与「历史数据自动补全展示信息」统一由通用组件
  // ProductPickGrid 负责；这里只负责把 ids / 展示缓存写回本模块的数据契约。

  return (
    <div className="dec-prod-pick">
      <div className="dec-prod-pick__mode">
        <RadioGroup
          type="button"
          buttonSize="small"
          value={selectType}
          onChange={(e: any) => onChange({ ...value, productSelectType: Number(e.target.value) })}
        >
          <Radio value={1}>{t('选商品')}</Radio>
          <Radio value={2}>{t('按分类')}</Radio>
          <Radio value={3}>{t('按标签')}</Radio>
        </RadioGroup>
      </div>

      {selectType === 1 && (
        <>
          <ProductPickGrid
            ids={ids}
            items={items}
            onChange={(nextIds, nextItems) => onChange({
              ...value,
              productIds: nextIds,
              productItems: nextItems,
            })}
            onRequestPick={() => setPickerVisible(true)}
          />
          {pickerVisible && (
            <ProductPickerModal
              visible
              selectedIds={ids}
              onOk={(nextIds, rows) => {
                onChange({
                  ...value,
                  productIds: nextIds,
                  productItems: mergePickedItems(nextIds, rows, items),
                });
                setPickerVisible(false);
              }}
              onCancel={() => setPickerVisible(false)}
            />
          )}
        </>
      )}

      {selectType === 2 && (
        <div className="dec-prod-pick__row">
          <label className="dec-prod-pick__field">
            <span>{t('商品分类')}</span>
            {(() => {
              const catId = Number(value.productCategoryId) || 0;
              // 优先用树算完整路径；树还没加载完时退回拍平选项并清掉缩进装饰符
              const pathLabel = catId
                ? findCategoryPath(categoryTree, catId)?.join(' / ') ||
                  categoryOptions
                    .find((o) => Number(o.value) === catId)
                    ?.label.replace(/^[\s　]*└─\s*/, '')
                : '';
              return (
                <div
                  className="dec-cat-field"
                  onClick={() => setCatPickerVisible(true)}
                  role="button"
                  title={pathLabel || undefined}
                >
                  <span className={`dec-cat-field__text${pathLabel ? '' : ' is-placeholder'}`}>
                    {pathLabel || t('选择分类')}
                  </span>
                  {catId > 0 && (
                    <button
                      type="button"
                      className="dec-cat-field__clear"
                      aria-label={t('清除')}
                      onClick={(e) => {
                        e.stopPropagation();
                        onChange({ ...value, productCategoryId: 0 });
                      }}
                    >
                      <IconClose size="small" />
                    </button>
                  )}
                  <IconChevronDown size="small" className="dec-cat-field__chev" />
                </div>
              );
            })()}
            {catPickerVisible && (
              <CategoryPickerDrawer
                visible
                currentId={Number(value.productCategoryId) || 0}
                tree={categoryTree}
                onOk={(id) => {
                  onChange({ ...value, productCategoryId: id });
                  setCatPickerVisible(false);
                }}
                onCancel={() => setCatPickerVisible(false)}
              />
            )}
          </label>
          <label className="dec-prod-pick__field">
            <span>{t('商品数量')}</span>
            <InputNumber
              value={value.productNumber ?? 10}
              onChange={(v) => onChange({ ...value, productNumber: v ?? 10 })}
              min={field.numberMin ?? 1}
              max={field.numberMax}
              style={{ width: '100%' }}
            />
          </label>
        </div>
      )}

      {selectType === 3 && (
        <div className="dec-prod-pick__row">
          <label className="dec-prod-pick__field">
            <span>{t('商品标签')}</span>
            {/* 后端只认 best/new/hot（映射 product.is_best/is_new/is_hot）。
                此前是自由文本输入，任意文字会被后端静默丢弃 → 按标签恒返回最新商品。 */}
            <RadioGroup
              type="button"
              buttonSize="small"
              value={value.productTag || 'best'}
              onChange={(e: any) => onChange({ ...value, productTag: String(e.target.value) })}
            >
              <Radio value="best">{t('精品推荐')}</Radio>
              <Radio value="new">{t('上新品')}</Radio>
              <Radio value="hot">{t('热销')}</Radio>
            </RadioGroup>
          </label>
          <label className="dec-prod-pick__field">
            <span>{t('商品数量')}</span>
            <InputNumber
              value={value.productNumber ?? 10}
              onChange={(v) => onChange({ ...value, productNumber: v ?? 10 })}
              min={field.numberMin ?? 1}
              max={field.numberMax}
              style={{ width: '100%' }}
            />
          </label>
        </div>
      )}
    </div>
  );
}

function FieldControl({
  field,
  moduleData,
  onChange,
}: {
  field: SettingFieldDef;
  moduleData: Record<string, any>;
  onChange: (next: Record<string, any>) => void;
}) {
  const { t } = useI18n();
  const categoryOptions = useCategoryOptions();
  const [pickerVisible, setPickerVisible] = useState(false);
  const [grouponVisible, setGrouponVisible] = useState(false);
  const [heroPicker, setHeroPicker] = useState<{ cardIndex: number; productIndex: number } | null>(null);
  const [groupPickerIndex, setGroupPickerIndex] = useState<number | null>(null);
  /** 商品分组里「选择商品」弹窗当前作用于第几组；不能用共享的 boolean，否则多组会同时弹窗 */
  const [groupProductPicker, setGroupProductPicker] = useState<number | null>(null);
  const [heroGroupPicker, setHeroGroupPicker] = useState<number | null>(null);
  const value = getPath(moduleData, field.key);

  const set = (v: unknown) => {
    let next = setPath(moduleData, field.key, v);
    if (field.onChangeExtra) next = field.onChangeExtra(next, v);
    onChange(next);
  };

  switch (field.kind) {
    case 'radio':
      if (field.iconRadio) {
        // 原版 el-radio-button + el-tooltip：以 ico-decorate 图标直观看模板/指示器样式
        const current = (value ?? field.options?.[0]?.value) as string | number | undefined;
        return (
          <div className="dec-icon-radio-group">
            {(field.options || []).map((o) => {
              const icon = field.optionIcons?.[o.value as string | number];
              const active = String(current) === String(o.value);
              return (
                <Tooltip key={String(o.value)} content={t(o.label)} position="bottom">
                  <button
                    type="button"
                    className={`dec-icon-radio${active ? ' is-active' : ''}`}
                    onClick={() => set(Number(o.value))}
                    aria-label={t(o.label)}
                    aria-pressed={active}
                  >
                    {icon ? <i className="ico-decorate" style={{ fontSize: 18 }}>{icon}</i> : t(o.label)}
                  </button>
                </Tooltip>
              );
            })}
          </div>
        );
      }
      return (
        <RadioGroup value={(value ?? field.options?.[0]?.value) as string | number | undefined} onChange={(e: any) => set(Number(e.target.value))}>
          {(field.options || []).map((o) => (
            <Radio key={String(o.value)} value={o.value}>{t(o.label)}</Radio>
          ))}
        </RadioGroup>
      );
    case 'switch':
      return <Switch checked={Number(value) === 1} onChange={(v) => set(v ? 1 : 0)} />;
    case 'select':
      return (
        <Select
          value={(value ?? field.options?.[0]?.value) as string | number | undefined}
          optionList={(field.options || []).map((o) => ({ label: t(o.label), value: o.value }))}
          style={{ width: 160 }}
          onChange={(v) => set(v)}
        />
      );
    case 'input':
      return <Input value={String(value ?? '')} onChange={(v) => set(v)} placeholder={field.placeholder ? t(field.placeholder) : undefined} />;
    case 'textarea':
      return <TextArea value={String(value ?? '')} onChange={(v: string) => set(v)} rows={field.rows ?? 2} maxLength={500} placeholder={field.placeholder ? t(field.placeholder) : undefined} />;
    case 'number':
      return (
        <InputNumber
          value={typeof value === 'number' ? value : undefined}
          onChange={(v) => set(v ?? undefined)}
          min={field.min}
          max={field.max}
          style={{ width: 160 }}
        />
      );
    case 'slider':
      return (
        <SliderWithInput
          value={typeof value === 'number' ? value : (field.defaultValue ?? 0)}
          min={field.min ?? 0}
          max={field.max ?? 30}
          step={1}
          onChange={(v) => set(v)}
        />
      );
    case 'squareTemplate': {
      const current = Number(value) || 1;
      const options = [
        { value: 1, label: '一行两个' },
        { value: 2, label: '一行三个' },
        { value: 3, label: '一行四个' },
        { value: 4, label: '左二右二' },
        { value: 5, label: '左一右二' },
        { value: 6, label: '左一右三' },
        { value: 7, label: '上一下二' },
      ];
      const picCount: Record<number, number> = { 1: 2, 2: 3, 3: 4, 4: 4, 5: 3, 6: 4, 7: 3 };
      const changeType = (nextType: number) => {
        let next = setPath(moduleData, field.key, nextType);
        const list = [...((getPath(next, 'picList') || []) as PicItem[])];
        const count = picCount[nextType] ?? 2;
        while (list.length < count) {
          list.push({ picId: '0', picUrl: '', picThumb: '', picName: '', picTitle: '', picLink: { link: '', name: '' } });
        }
        if (list.length > count) list.length = count;
        next = setPath(next, 'picList', list);
        onChange(next);
      };
      return (
        <div className="square-template">
          <div className="square-template__value">{t(options.find((item) => item.value === current)?.label || t('一行两个'))}</div>
          <div className="square-template__options">
            {options.map((item) => (
              <Tooltip key={item.value} content={t(item.label)} position="top">
                <button
                  type="button"
                  className={`square-template__option${current === item.value ? ' is-active' : ''}`}
                  onClick={() => changeType(item.value)}
                >
                  <span className={`square-thumb square-thumb--${item.value}`}>
                    <i /><i /><i /><i />
                  </span>
                </button>
              </Tooltip>
            ))}
          </div>
        </div>
      );
    }
    case 'colorPair': {
      const secondValue = field.secondKey ? getPath(moduleData, field.secondKey) : '';
      const setSecond = (v: unknown) => {
        if (!field.secondKey) return;
        onChange(setPath(moduleData, field.secondKey, v));
      };
      // 双色对外壳走共享 components/ColorField 的 ColorPairField（重置同时复位两色，行为等价）
      return (
        <ColorPairField
          value={String(value || '')}
          secondValue={String(secondValue || '')}
          onChange={(v) => set(v)}
          onSecondChange={(v) => setSecond(v)}
          resetValue={String(field.resetValue ?? '')}
          resetValueSecond={String(field.resetValueSecond ?? '')}
        />
      );
    }
    case 'color':
      // 单色外壳走共享 components/ColorField（重置链 + AlphaColorInput；重置值口径不变）
      return (
        <SharedColorField
          value={String(value || '')}
          onChange={(v) => set(v)}
          resetValue={String(field.resetValue ?? '')}
        />
      );
    case 'picList':
      return <PicListEditor value={(value as PicItem[]) || []} onChange={(v) => set(v)} showDesc={field.picDesc} addText={field.picAddTextFor ? field.picAddTextFor(moduleData) : field.picAddText} showLink={!field.picNoLink} showText={!field.picNoText} max={field.picMaxFor ? field.picMaxFor(moduleData) : field.picMax} />;
    case 'pic':
      return <SinglePicField value={value} onChange={(v) => set(v)} resetValue={field.resetValue} />;
    case 'link': {
      const link = (value && typeof value === 'object' ? value : {}) as Record<string, any>;
      return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <LinkField value={link} onChange={(next) => set(next)} />
          {link.title !== undefined && (
            <Input value={link.title || ''} onChange={(v) => set({ ...link, title: v })} placeholder={t('链接文字(可选)')} />
          )}
        </div>
      );
    }
    case 'linkList': {
      const list = (Array.isArray(value) ? value : []) as Array<{ link?: { link?: string; title?: string }; linkTitle?: string }>;
      return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {list.map((item, index) => (
            <div key={index} style={{ display: 'flex', gap: 6 }}>
              <LinkField value={item.link} onChange={(link) => set(list.map((it, i) => (i === index ? { ...it, link } : it)))} />
              <Input value={item.linkTitle || ''} onChange={(v) => set(list.map((it, i) => (i === index ? { ...it, linkTitle: v } : it)))} placeholder={t('名称')} style={{ width: 110 }} />
              <Button size="small" theme="borderless" type="danger" icon={<IconClose />} onClick={() => set(list.filter((_, i) => i !== index))} />
            </div>
          ))}
          <Button size="small" icon={<IconPlus />} onClick={() => set([...list, { link: { link: '', title: '' }, linkTitle: '' }])}>
            {t('添加链接')}
          </Button>
        </div>
      );
    }
    case 'category':
      return (
        <Select
          value={typeof value === 'number' && value > 0 ? value : undefined}
          optionList={categoryOptions}
          showClear
          filter
          style={{ width: '100%' }}
          placeholder={t('选择商品分类')}
          onChange={(v) => set(v ? Number(v) : 0)}
        />
      );
    case 'categoryMulti': {
      const arr = Array.isArray(value) ? value : typeof value === 'number' && value > 0 ? [value] : [];
      return (
        <Select
          value={arr as unknown as string[]}
          optionList={categoryOptions}
          multiple
          showClear
          filter
          style={{ width: '100%' }}
          placeholder={t('选择商品分类(可多选)')}
          onChange={(v) => set(((v as unknown as number[]) || []).map(Number))}
        />
      );
    }
    case 'productIds': {
      const ids = (Array.isArray(value) ? value : []) as number[];
      return (
        <div>
          <Button icon={<IconPlus />} onClick={() => setPickerVisible(true)}>{t('选择商品（已选 {count}）', { count: ids.length })}</Button>
          {ids.length > 0 && (
            <div style={{ marginTop: 6, display: 'flex', flexWrap: 'wrap', gap: 4 }}>
              {ids.map((id) => <Tag key={id} size="small">#{id}</Tag>)}
            </div>
          )}
          {pickerVisible && (
            <ProductPickerModal visible selectedIds={ids} onOk={(nextIds) => { set(nextIds); setPickerVisible(false); }} onCancel={() => setPickerVisible(false)} />
          )}
        </div>
      );
    }
    case 'products': {
      const products = (value && typeof value === 'object' ? value : productsDefaults()) as Record<string, any>;
      const setProducts = (next: Record<string, any>) => set(next);
      return (
        <ProductsSettingField value={products} onChange={setProducts} field={field} categoryOptions={categoryOptions} />
      );
    }
    case 'groupon': {
      const item = value as Record<string, any> | undefined | null;
      return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {item ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <Tag color="blue">{item.name || t('活动#{id}', { id: item.grouponPromotionId })}</Tag>
              <Button size="small" theme="borderless" type="danger" onClick={() => set(null)}>{t('移除')}</Button>
            </div>
          ) : (
            <Text type="tertiary" size="small">{t('未选择活动')}</Text>
          )}
          <Button icon={<IconPlus />} onClick={() => setGrouponVisible(true)}>{t('选择拼团活动')}</Button>
          {grouponVisible && (
            <GrouponPickerModal visible current={item || null} onOk={(next) => { set(next); setGrouponVisible(false); }} onCancel={() => setGrouponVisible(false)} />
          )}
        </div>
      );
    }
    case 'title': {
      const titleData = (value && typeof value === 'object' ? value : standardTitle()) as Record<string, any>;
      // rootKey:'' —— fields 的 key 与 showIf 都基于「标题对象本身」，与下面 data={titleData} 一致。
      // 不要再套 title. 前缀，否则 showIf 恒 false、标题子项全部隐藏。
      return (
        <SettingsFieldRows
          fields={titleFields({
            rootKey: '',
            moreType: !field.titleHideMoreType,
            verticalPadding: !field.titleHideVerticalPadding,
          })}
          data={titleData}
          onChange={(next) => set(next)}
        />
      );
    }
    case 'container': {
      if (field.containerType && field.containerType !== 'frame') {
        const group = (value && typeof value === 'object' ? value : {}) as Record<string, any>;
        return <SettingsFieldRows fields={CONTAINER_VARIANTS[field.containerType] || []} data={group} onChange={(next) => set(next)} />;
      }
      const frame = (getPath(moduleData, field.key) && typeof getPath(moduleData, field.key) === 'object' ? getPath(moduleData, field.key) : standardFrame()) as Record<string, any>;
      return (
        <SettingsFieldRows
          fields={containerFields().map((f) => ({ ...f, key: f.key.replace('frame.', '') }))}
          data={frame}
          onChange={(next) => set(next)}
        />
      );
    }
    case 'heroCards': {
      const cards = (Array.isArray(value) ? value : []) as Array<Record<string, any>>;
      const updateCard = (index: number, updater: (c: Record<string, any>) => Record<string, any>) => {
        set(cards.map((c, i) => (i === index ? updater(c) : c)));
      };
      const updateProduct = (cardIndex: number, productIndex: number, patch: Record<string, any>) => {
        updateCard(cardIndex, (c) => ({
          ...c,
          products: ((c.products || []) as Array<Record<string, any>>).map((p, i) => (i === productIndex ? { ...p, ...patch } : p)),
        }));
      };
      const removeProduct = (cardIndex: number, productIndex: number) => {
        updateCard(cardIndex, (c) => ({
          ...c,
          products: ((c.products || []) as Array<Record<string, any>>).filter((_, i) => i !== productIndex),
        }));
      };
      return (
        <div>
          {cards.map((card, ci) => {
            const source = card.productSource || 'custom';
            return (
              <div className="dec-hero-card-editor" key={ci}>
                <div className="dec-hero-card-editor__header">{t('卡位 {index}', { index: ci + 1 })}</div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('标题')}</div>
                  <div className="dec-hero-card-editor__con">
                    <Input
                      value={card.moduleTitle || ''}
                      placeholder={t('请输入标题')}
                      onChange={(v) => updateCard(ci, (c) => ({ ...c, moduleTitle: v }))}
                    />
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('子标题')}</div>
                  <div className="dec-hero-card-editor__con">
                    <Input
                      value={card.moduleSubTitle || ''}
                      placeholder={t('文案')}
                      onChange={(v) => updateCard(ci, (c) => ({ ...c, moduleSubTitle: v }))}
                    />
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('子标题背景')}</div>
                  <div className="dec-hero-card-editor__con">
                    <div className="dec-color-group dec-color-group--dual">
                      <div className="dec-color-button">
                        <a
                          className="dec-color-reset"
                          onClick={() => updateCard(ci, (c) => ({ ...c, moduleSubTitleBgColorA: '#ff4e4e', moduleSubTitleBgColorB: '#e1251b' }))}
                        >
                          {t('重置')}
                        </a>
                        <AlphaColorInput value={card.moduleSubTitleBgColorA || '#ff4e4e'} onChange={(v) => updateCard(ci, (c) => ({ ...c, moduleSubTitleBgColorA: v }))} />
                        <span className="dec-color-sep">-</span>
                        <AlphaColorInput value={card.moduleSubTitleBgColorB || '#e1251b'} onChange={(v) => updateCard(ci, (c) => ({ ...c, moduleSubTitleBgColorB: v }))} />
                      </div>
                    </div>
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('渐变方向')}</div>
                  <div className="dec-hero-card-editor__con">
                    <RadioGroup
                      type="button"
                      value={card.moduleSubTitleBgType || 'leftRight'}
                      onChange={(e: any) => updateCard(ci, (c) => ({ ...c, moduleSubTitleBgType: e.target.value }))}
                    >
                      <Radio value="leftRight">{t('左右')}</Radio>
                      <Radio value="upDown">{t('上下')}</Radio>
                      <Radio value="diagonal">{t('斜向')}</Radio>
                    </RadioGroup>
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('链接')}</div>
                  <div className="dec-hero-card-editor__con">
                    <LinkField value={card.link} onChange={(link) => updateCard(ci, (c) => ({ ...c, link }))} />
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('显示价格')}</div>
                  <div className="dec-hero-card-editor__con">
                    <Switch checked={String(card.showPrice) !== '0'} onChange={(v) => updateCard(ci, (c) => ({ ...c, showPrice: v ? 1 : 0 }))} />
                  </div>
                </div>

                <div className="dec-hero-card-editor__row">
                  <div className="dec-hero-card-editor__label">{t('商品来源')}</div>
                  <div className="dec-hero-card-editor__con">
                    <RadioGroup
                      type="button"
                      value={source}
                      onChange={(e: any) => updateCard(ci, (c) => ({
                        ...c,
                        productSource: e.target.value,
                        productGroupId: e.target.value === 'custom' ? undefined : c.productGroupId,
                        productGroupName: e.target.value === 'custom' ? undefined : c.productGroupName,
                      }))}
                    >
                      <Radio value="custom">{t('自定义商品')}</Radio>
                      <Radio value="productGroup">{t('商品分组')}</Radio>
                    </RadioGroup>
                  </div>
                </div>

                {source === 'custom' ? (
                  <div className="dec-hero-products-edit">
                    <div className="dec-hero-products-title">{t('选择商品')}</div>
                    <div className="dec-hero-products-list">
                      {((card.products || []) as Array<Record<string, any>>).slice(0, 2).map((prod, pi) => (
                        <div className="dec-hero-product-item" key={pi}>
                          <div className="dec-hero-product-item__left">
                            <div className="dec-hero-product-img-box" title={prod.productName || t('点击选择商品')} onClick={() => setHeroPicker({ cardIndex: ci, productIndex: pi })}>
                              {prod.pic?.picThumb || prod.pic?.picUrl ? (
                                <img className="dec-hero-product-img" src={String(prod.pic?.picThumb || prod.pic?.picUrl)} alt="" />
                              ) : (
                                <div className="dec-hero-product-img-empty"><IconPlus /></div>
                              )}
                              {(prod.productId || prod.productName || prod.pic?.picUrl) ? (
                                <span
                                  className="dec-hero-product-delete"
                                  onClick={(event) => { event.stopPropagation(); removeProduct(ci, pi); }}
                                >
                                  <IconClose size="small" />
                                </span>
                              ) : null}
                            </div>
                          </div>
                          <div className="dec-hero-product-item__right">
                            <div className="dec-hero-product-row">
                              <Input
                                value={prod.label || ''}
                                placeholder={t('标签文案')}
                                size="small"
                                onChange={(v) => updateProduct(ci, pi, { label: v })}
                              />
                            </div>
                            <div className="dec-hero-product-row dec-hero-product-row-bg-color">
                              <span className="dec-hero-product-label">{t('标签背景')}</span>
                              <AlphaColorInput value={prod.labelBgColorA || '#ff9000'} onChange={(v) => updateProduct(ci, pi, { labelBgColorA: v })} />
                              <AlphaColorInput value={prod.labelBgColorB || '#ff4e4e'} onChange={(v) => updateProduct(ci, pi, { labelBgColorB: v })} />
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                    <div className="dec-hero-products-tip">{t('提示：点击图片选择商品，可自定义标签文案和价格')}</div>
                  </div>
                ) : (
                  <div className="dec-hero-products-edit">
                    <div className="dec-hero-products-title">{t('选择商品分组')}</div>
                    <div className="dec-hero-group-select">
                      <Button size="small" icon={<IconPlus />} onClick={() => setHeroGroupPicker(ci)}>
                        {card.productGroupName ? t('更换商品分组') : t('添加商品分组')}
                      </Button>
                      {card.productGroupName ? (
                        <div className="dec-hero-group-info">
                          <div className="dec-hero-group-info__row">
                            <span className="dec-hero-group-info__name">{String(card.productGroupName)}</span>
                            <a className="dec-hero-group-info__clear" onClick={() => updateCard(ci, (c) => ({ ...c, productGroupId: undefined, productGroupName: undefined }))}>{t('清除')}</a>
                          </div>
                        </div>
                      ) : null}
                    </div>
                    <div className="dec-hero-products-tip">{t('提示：选择商品分组后，将自动获取该分组前 2 个商品展示')}</div>
                  </div>
                )}
              </div>
            );
          })}
          {heroGroupPicker !== null && (
            <ProductGroupPickerModal
              visible
              current={{ productGroupId: cards[heroGroupPicker]?.productGroupId, productGroupName: cards[heroGroupPicker]?.productGroupName } as any}
              onOk={(group) => {
                updateCard(heroGroupPicker, (c) => ({ ...c, productGroupId: group.productGroupId, productGroupName: group.productGroupName }));
                setHeroGroupPicker(null);
              }}
              onCancel={() => setHeroGroupPicker(null)}
            />
          )}
          {heroPicker && (
            <ProductPickerModal
              visible
              multiple={false}
              selectedIds={[]}
              onOk={(ids, rows) => {
                const row = rows.find((item) => item.productId === ids[0]) || rows[0];
                if (row) {
                  updateCard(heroPicker.cardIndex, (c) => ({
                    ...c,
                    products: (c.products || []).map((x: any, xi: number) => (xi === heroPicker.productIndex ? {
                      ...x,
                      productId: row.productId,
                      productName: row.productName,
                      pic: { picUrl: row.picUrl || row.picThumb || '', picThumb: row.picThumb || row.picUrl || '' },
                      price: row.productPrice !== undefined ? `¥${row.productPrice}` : x.price,
                    } : x)),
                  }));
                }
                setHeroPicker(null);
              }}
              onCancel={() => setHeroPicker(null)}
            />
          )}
        </div>
      );
    }
    case 'hotarea': {
      const pics = (getPath(moduleData, 'picList') || []) as Array<PicItem & { hotarea?: Array<Record<string, any>> }>;
      return (
        <HotareaPainter
          picList={pics}
          onChange={(nextPics) => onChange(setPath(moduleData, 'picList', nextPics))}
        />
      );
    }
    case 'groupList': {
      const groups = (Array.isArray(value) ? value : []) as Array<Record<string, any>>;
      // 「选择商品」卡片网格复用通用组件 ProductPickGrid（compact 适配分组内的窄容器）。
      // 本分支只负责把组件返回的 ids / 展示缓存写回分组的 productIds / productList
      // ——预览组件按 productList 取数，字段名不能改。
      return (
        <div className="dec-group-list">
          {groups.map((g, index) => {
            const groupIds = (g.productIds || []) as number[];
            return (
            <div key={index} className="dec-group-list__item">
              <div className="dec-group-list__title">{t('第【{index}】组', { index: index + 1 })}{index === 0 ? t('（固定，不可删除/拖动）') : ''}</div>
              <div>
                <Text type="tertiary" size="small" style={{ display: 'block', marginBottom: 4 }}>{t('标题')}</Text>
                <Input value={g.title || ''} onChange={(v) => set(groups.map((x, i) => (i === index ? { ...x, title: v } : x)))} placeholder={t('请输入标题内容')} />
              </div>
              <div>
                <Text type="tertiary" size="small" style={{ display: 'block', marginBottom: 4 }}>{t('标题图片')}</Text>
                <FieldControl
                  field={{ kind: 'pic', key: 'titlePic', label: '标题图片' }}
                  moduleData={g}
                  onChange={(nextGroup) => set(groups.map((x, i) => (i === index ? nextGroup : x)))}
                />
              </div>
              <div className="dec-group-list__mode">
                <RadioGroup type="button" buttonSize="small" value={g.sourceType || 'products'} onChange={(e: any) => {
                  const type = e.target.value;
                  if ((g.sourceType || 'products') === type) return;
                  // 对齐原版 handleSourceTypeChange：切换来源时清空对应数据
                  set(groups.map((x, i) => (i === index
                    ? (type === 'products'
                      ? { ...x, sourceType: type, productGroupId: undefined, productGroupName: undefined }
                      : { ...x, sourceType: type, productList: [] })
                    : x)));
                }}>
                  <Radio value="products">{t('选择商品')}</Radio>
                  <Radio value="productGroup">{t('商品分组')}</Radio>
                </RadioGroup>
              </div>
              {(g.sourceType || 'products') === 'products' ? (
                <ProductPickGrid
                  compact
                  ids={groupIds}
                  items={(g.productList || []) as any[]}
                  addText={t('选择商品')}
                  onChange={(nextIds, nextItems) => set(groups.map((x, i) => (
                    i === index ? { ...x, productIds: nextIds, productList: nextItems } : x
                  )))}
                  onRequestPick={() => setGroupProductPicker(index)}
                />
              ) : (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <button type="button" className="dec-prod-pick__add dec-group-list__group-add" onClick={() => setGroupPickerIndex(index)}>
                    <IconPlus />
                    <span>{g.productGroupName ? t('更换商品分组') : t('添加商品分组')}</span>
                  </button>
                  {g.productGroupName ? (
                    <div className="dec-group-list__selected">
                      <span className="dec-group-list__selected-name">{g.productGroupName}</span>
                      <button
                        type="button"
                        className="dec-group-list__selected-remove"
                        aria-label={t('移除')}
                        onClick={() => set(groups.map((x, i) => (i === index ? { ...x, productGroupId: undefined, productGroupName: undefined } : x)))}
                      >
                        ×
                      </button>
                    </div>
                  ) : null}
                </div>
              )}
              {index > 0 && (
                <Button size="small" theme="borderless" type="danger" icon={<IconClose />} onClick={() => set(groups.filter((_, i) => i !== index))}>
                  {t('删除该组')}
                </Button>
              )}
            </div>
            );
          })}
          <Button size="small" icon={<IconPlus />} onClick={() => set([...groups, { sourceType: 'products', title: '', titlePic: { picUrl: '', picThumb: '' }, titleLink: { link: '', name: '' }, bannerPic: { picUrl: '', picThumb: '' }, bannerLink: { link: '', name: '' }, productIds: [], productList: [] }])}>
            {t('添加一组')}
          </Button>
          {groupPickerIndex !== null && (
            <ProductGroupPickerModal
              visible
              current={groups[groupPickerIndex]}
              onOk={(group) => {
                set(groups.map((x, i) => (i === groupPickerIndex ? { ...x, productGroupId: group.productGroupId, productGroupName: group.productGroupName, sourceType: 'productGroup' } : x)));
                setGroupPickerIndex(null);
              }}
              onCancel={() => setGroupPickerIndex(null)}
            />
          )}
          {/* 选品弹窗放在分组列表外层，按组索引只挂载一个；原先挂在每组内部的共享 boolean 会让多组同时弹窗 */}
          {groupProductPicker !== null && (
            <ProductPickerModal
              visible
              selectedIds={(groups[groupProductPicker]?.productIds || []) as number[]}
              onOk={(nextIds, rows) => {
                const target = groups[groupProductPicker];
                set(groups.map((x, i) => (i === groupProductPicker
                  ? {
                      ...x,
                      productIds: nextIds,
                      productList: mergePickedItems(nextIds, rows, (target?.productList || []) as any[]),
                    }
                  : x)));
                setGroupProductPicker(null);
              }}
              onCancel={() => setGroupProductPicker(null)}
            />
          )}
        </div>
      );
    }
    default:
      return null;
  }
}















