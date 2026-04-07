export type WatchAsset = 'BTC' | 'ETH' | 'SOL';

interface Props {
  value: WatchAsset;
  onChange: (v: WatchAsset) => void;
}

const OPTIONS: { id: WatchAsset; label: string }[] = [
  { id: 'BTC', label: 'Bitcoin' },
  { id: 'ETH', label: 'Ethereum' },
  { id: 'SOL', label: 'Solana' },
];

export default function ChainSwitcher({ value, onChange }: Props) {
  return (
    <div
      role="group"
      aria-label="Watch chain"
      style={{
        display: 'flex',
        gap: '0.35rem',
        background: 'rgba(0,0,0,0.25)',
        padding: '0.35rem',
        borderRadius: 14,
        border: '1px solid var(--border-light)',
        flexWrap: 'wrap',
      }}
    >
      {OPTIONS.map((o) => (
        <button
          key={o.id}
          type="button"
          title={o.label}
          onClick={() => onChange(o.id)}
          style={{
            padding: '0.45rem 0.85rem',
            borderRadius: 10,
            border: 'none',
            font: 'inherit',
            fontWeight: 600,
            fontSize: '0.8rem',
            cursor: 'pointer',
            background: value === o.id ? 'var(--accent)' : 'transparent',
            color: value === o.id ? '#fff' : 'var(--text-muted)',
            transition: 'background 0.2s, color 0.2s',
          }}
        >
          {o.id}
        </button>
      ))}
    </div>
  );
}
