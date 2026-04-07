import type { PriceInfo } from '../types';

interface Props {
  price?: PriceInfo | null;
}

export default function PriceTicker({ price }: Props) {
  if (!price) return <div className="price-ticker">--</div>;
  const p = Number(price.price || 0);
  const valid = price.valid !== undefined ? price.valid : true;
  const color = valid ? 'var(--bid-color)' : 'var(--ask-color)';
  return (
    <div className="price-ticker" style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
      <div style={{ fontSize: '0.9rem', color: 'var(--muted)', textTransform: 'uppercase' }}>{price.base}/{price.quote}</div>
      <div style={{ fontWeight: 700, fontSize: '1.1rem', color }}>{p.toFixed(2)}</div>
      {price.source && <div style={{ fontSize: '0.8rem', color: 'var(--muted)' }}>{price.source}</div>}
      {!valid && <div style={{ fontSize: '0.8rem', color: 'var(--ask-color)' }}>invalid</div>}
    </div>
  );
}
