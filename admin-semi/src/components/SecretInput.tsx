import { Input } from '@douyinfe/semi-ui';

interface SecretInputProps {
  value?: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

export default function SecretInput({ value = '', placeholder, onChange }: SecretInputProps) {
  return (
    <Input
      value={value}
      placeholder={placeholder}
      mode="password"
      onChange={onChange}
    />
  );
}
