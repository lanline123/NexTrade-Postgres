// IconPark 全量图标目录（2658 个，@icon-park/svg 1.4.2 生成）
// 静态文件 /iconpark-catalog.json（pc-semi/public/），运行时按需拉取、内存缓存。
// 每条：kebab 名 / 中文名 / 中文分类 / 中文关键词 / SVG 内部标记
// （viewBox 统一 0 0 48 48，描边色已归一为 currentColor，大小与颜色由外层控制）。

export interface IconParkEntry {
  name: string;
  title: string;
  category: string;
  tags: string;
  inner: string;
}

let entries: IconParkEntry[] | null = null;
let byName: Map<string, IconParkEntry> | null = null;
let loading: Promise<IconParkEntry[]> | null = null;

export function loadIconParkCatalog(): Promise<IconParkEntry[]> {
  if (entries) return Promise.resolve(entries);
  if (!loading) {
    loading = fetch('/iconpark-catalog.json')
      .then((r) => {
        if (!r.ok) throw new Error(`iconpark catalog ${r.status}`);
        return r.json();
      })
      .then((data: any) => {
        const rows: any[] = Array.isArray(data?.icons) ? data.icons : [];
        entries = rows.map((r) => ({
          name: String(r[0] || ''),
          title: String(r[1] || ''),
          category: String(r[2] || ''),
          tags: String(r[3] || ''),
          inner: String(r[4] || ''),
        }));
        byName = new Map(entries.map((e) => [e.name, e]));
        return entries;
      })
      .catch((e) => {
        loading = null;
        throw e;
      });
  }
  return loading;
}

export function getIconParkEntry(name: string): IconParkEntry | undefined {
  return byName?.get(name);
}

export function isIconParkValue(value: string): boolean {
  return typeof value === 'string' && value.startsWith('iconpark:');
}

export function iconParkNameOf(value: string): string {
  return isIconParkValue(value) ? value.slice('iconpark:'.length) : '';
}

export function iconParkSvg(entry: IconParkEntry, size: number): string {
  return `<svg viewBox="0 0 48 48" width="${size}" height="${size}" fill="none" xmlns="http://www.w3.org/2000/svg">${entry.inner}</svg>`;
}
