import React, { useEffect, useState } from 'react';
import {
  getIconParkEntry,
  iconParkNameOf,
  iconParkSvg,
  isIconParkValue,
  loadIconParkCatalog,
} from './iconpark';

/**
 * 楼层图标渲染（floorIcoFont 值）：`iconpark:<name>` → IconPark 内嵌 SVG
 * （目录懒加载，加载前占位同尺寸空盒避免跳动）。旧 iconfont 类名值已随
 * 2026-11-15 数据迁移清零，不再渲染。
 */
export default function FloorIcon({
  value,
  size = 18,
}: {
  value?: string;
  size?: number;
}) {
  const name = iconParkNameOf(value || '');
  const [, setTick] = useState(0);

  useEffect(() => {
    if (name && !getIconParkEntry(name)) {
      let on = true;
      loadIconParkCatalog()
        .then(() => {
          if (on) setTick((x) => x + 1);
        })
        .catch(() => undefined);
      return () => {
        on = false;
      };
    }
    return undefined;
  }, [name]);

  if (!value || !isIconParkValue(value)) return null;
  const entry = getIconParkEntry(name);
  if (!entry) {
    return <span style={{ display: 'inline-block', width: size, height: size }} />;
  }
  return (
    <span
      style={{ display: 'inline-flex', width: size, height: size, lineHeight: 0, flexShrink: 0 }}
      dangerouslySetInnerHTML={{ __html: iconParkSvg(entry, size) }}
    />
  );
}
