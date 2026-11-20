/**
 * 店铺标签徽章（本平台新增，论坛帖 207878955346177229）。
 * 平台分配的展示标签，紧贴店铺名渲染；颜色取标签库配置，
 * 未配色时回退品牌红。老数据无 labels 字段时不渲染。
 */
export interface ShopLabelChipData {
  labelId?: number;
  labelName?: string;
  color?: string;
}

export default function ShopLabelChips({
  labels,
  style,
}: {
  labels?: ShopLabelChipData[] | null;
  style?: React.CSSProperties;
}) {
  const list = (labels || []).filter((l) => l && l.labelName);
  if (list.length === 0) return null;
  return (
    <span
      style={{
        display: 'inline-flex',
        flexWrap: 'wrap',
        gap: 4,
        marginLeft: 8,
        verticalAlign: 'middle',
        ...style,
      }}
    >
      {list.map((label, index) => (
        <span
          key={label.labelId ?? index}
          style={{
            display: 'inline-block',
            padding: '0 6px',
            height: 18,
            lineHeight: '18px',
            fontSize: 11,
            fontWeight: 'normal',
            color: '#fff',
            background: label.color || '#E93B3D',
            whiteSpace: 'nowrap',
          }}
        >
          {label.labelName}
        </span>
      ))}
    </span>
  );
}
