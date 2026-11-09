import { useRef } from 'react';

export interface PaymentOrderOption {
  label: string;
  value: string;
}

interface PaymentOrderListProps {
  /** 逗号分隔的支付方式 code，顺序即排序结果。 */
  value: string;
  options: PaymentOrderOption[];
  onChange: (next: string) => void;
}

/** 支付顺序拖拽排序控件：后端下发逗号串，未覆盖的选项按默认顺序补在末尾。 */
export default function PaymentOrderList({ value, options, onChange }: PaymentOrderListProps) {
  const dragIndex = useRef<number | null>(null);

  const ordered = (() => {
    const known = new Map(options.map((item) => [item.value.toLowerCase(), item] as const));
    const parts = String(value ?? '')
      .split(',')
      .map((item) => known.get(item.trim().toLowerCase()))
      .filter((item): item is PaymentOrderOption => Boolean(item));
    const unique = [...new Map(parts.map((item) => [item.value, item] as const)).values()];
    options.forEach((item) => {
      if (!unique.some((entry) => entry.value === item.value)) unique.push(item);
    });
    return unique;
  })();

  const handleDragStart = (index: number) => {
    dragIndex.current = index;
  };

  const handleDrop = (index: number) => {
    const from = dragIndex.current;
    dragIndex.current = null;
    if (from === null || from === index) return;
    const next = [...ordered];
    const [moved] = next.splice(from, 1);
    next.splice(index, 0, moved);
    onChange(next.map((item) => item.value).join(','));
  };

  return (
    <div className="payment-order-list" data-widget="payment-order-drag">
      {ordered.map((item, index) => (
        <div
          key={item.value}
          className="payment-order-item"
          draggable
          onDragStart={() => handleDragStart(index)}
          onDragOver={(event) => event.preventDefault()}
          onDrop={() => handleDrop(index)}
        >
          <span className="payment-order-drag-handle" title="拖拽排序">
            ⋮⋮
          </span>
          <span className="payment-order-label">{item.label}</span>
        </div>
      ))}
    </div>
  );
}
