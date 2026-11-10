interface PriceProps {
  value: string | number | undefined;
  size?: number;
  decimalSize?: number;
  weight?: number;
  muted?: boolean;
  strike?: boolean;
}

function split(value: string | number | undefined): [string, string] {
  const amount = Number(value ?? 0);
  const text = (Number.isFinite(amount) ? amount : 0).toFixed(2);
  const [integer, decimal] = text.split('.');
  return [integer, decimal ?? '00'];
}

export default function Price({
  value,
  size = 16,
  decimalSize,
  weight = 600,
  muted = false,
  strike = false,
}: PriceProps) {
  const [integer, decimal] = split(value);
  const style = {
    fontSize: size,
    fontWeight: weight,
    color: muted ? '#999' : undefined,
    textDecoration: strike ? 'line-through' : undefined,
  };

  return (
    <div className="price align-bottom">
      <div className="price-row">
        <div className="align-bottom util" style={style}>
          ¥
        </div>
        <div className="num integer" style={style}>
          {integer}
        </div>
        <div className="num decimal" style={{ ...style, fontSize: decimalSize ?? size }}>
          .{decimal}
        </div>
      </div>
    </div>
  );
}
