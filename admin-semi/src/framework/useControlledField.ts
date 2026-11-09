import { useEffect, useState } from 'react';

/**
 * 统一受控字段状态：
 * - 外部值变化时同步到本地
 * - 点击/输入时立即更新本地视觉状态
 * - 同时把值写回 Form
 */
export function useControlledField<T>(externalValue: T, onChange: (value: T) => void) {
  const [value, setValue] = useState<T>(externalValue);

  useEffect(() => {
    setValue(externalValue);
  }, [JSON.stringify(externalValue)]);

  const update = (next: T) => {
    setValue(next);
    onChange(next);
  };

  return [value, update] as const;
}
