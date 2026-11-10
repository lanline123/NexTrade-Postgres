import { useEffect, useMemo, useState } from 'react';
import { Cascader, Input, Select } from '@douyinfe/semi-ui';
import { apiGet } from '../api';
import { useI18n } from '../i18n';
import type { ConfigRendererContext } from '../pages/AutoConfigPage';

interface RegionNode {
  regionId: number;
  regionName: string;
  children?: RegionNode[];
}

interface CascaderNode {
  label: string;
  value: number;
  children?: CascaderNode[];
}

let treeCache: RegionNode[] | null = null;
let treePromise: Promise<RegionNode[]> | null = null;

function loadRegionTree(): Promise<RegionNode[]> {
  if (treeCache) return Promise.resolve(treeCache);
  if (!treePromise) {
    treePromise = apiGet<RegionNode[]>('/setting/region/getAllRegionTree')
      .then((result) => {
        treeCache = Array.isArray(result) ? result : [];
        return treeCache;
      })
      .catch(() => [])
      .finally(() => {
        treePromise = null;
      });
  }
  return treePromise;
}

function toCascader(nodes: RegionNode[]): CascaderNode[] {
  return nodes.map((node) => ({
    label: node.regionName,
    value: node.regionId,
    children: node.children?.length ? toCascader(node.children) : undefined,
  }));
}

function useRegionTree(): { tree: RegionNode[]; ready: boolean } {
  const [state, setState] = useState(() => ({
    tree: treeCache || [],
    ready: treeCache !== null,
  }));
  useEffect(() => {
    let alive = true;
    void loadRegionTree().then((data) => {
      if (alive) setState({ tree: data, ready: true });
    });
    return () => {
      alive = false;
    };
  }, []);
  return state;
}

/** 对齐旧版 GlobalSetting.vue：默认国家下拉，选项为地区树的一级节点，0 表示不选择。 */
export function CountrySelectField({ value, update }: ConfigRendererContext) {
  const { t } = useI18n();
  const { tree, ready } = useRegionTree();
  const options = useMemo(
    () => [
      { label: t('不选择'), value: 0 },
      ...tree.map((node) => ({ label: node.regionName, value: node.regionId })),
    ],
    [tree, t],
  );
  if (!ready) return <Input disabled style={{ width: 300 }} placeholder={t('加载中')} />;
  return (
    <Select
      style={{ width: 300 }}
      value={Number(value) || 0}
      optionList={options}
      onChange={(next) => update('defaultCountry', next)}
    />
  );
}

/** 对齐旧版 SelectRegion：省市区级联，保存为逗号分隔的 regionId 路径。 */
export function ShopRegionCascaderField({ value, update }: ConfigRendererContext) {
  const { t } = useI18n();
  const { tree, ready } = useRegionTree();
  const treeData = useMemo(() => toCascader(tree), [tree]);
  const selected = useMemo(
    () =>
      String(value ?? '')
        .split(',')
        .filter(Boolean)
        .map(Number)
        .filter((id) => Number.isFinite(id)),
    [value],
  );
  if (!ready) return <Input disabled style={{ width: 300 }} placeholder={t('加载中')} />;
  return (
    <Cascader
      style={{ width: 300 }}
      treeData={treeData}
      value={selected.length ? selected : undefined}
      placeholder={t('请选择地区')}
      showClear
      onChange={(next) =>
        update('shopDefaultRegions', Array.isArray(next) ? next.join(',') : '')
      }
    />
  );
}
