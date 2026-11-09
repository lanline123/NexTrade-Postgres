import { useEffect, useState } from 'react';
import { Select } from '@douyinfe/semi-ui';
import { apiGet } from '../api';

interface RegionSelectProps {
  value?: number | string;
  onChange: (value: number | string) => void;
  placeholder?: string;
}

interface RegionItem {
  regionId: number;
  regionName: string;
}

export default function RegionSelect({ value, onChange, placeholder = '请选择地区' }: RegionSelectProps) {
  const [options, setOptions] = useState<RegionItem[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setLoading(true);
    apiGet<{ records?: RegionItem[] }>('/setting/region/list', { parentId: 0, page: 1, size: 100 })
      .then((result) => setOptions(result?.records || []))
      .catch(() => undefined)
      .finally(() => setLoading(false));
  }, []);

  return (
    <Select
      value={value === undefined || value === null || value === '' ? undefined : value}
      loading={loading}
      placeholder={placeholder}
      filter
      onChange={(next) => onChange(next as number | string)}
      style={{ width: '100%' }}
      optionList={options.map((item) => ({
        label: item.regionName,
        value: item.regionId,
      }))}
    />
  );
}
