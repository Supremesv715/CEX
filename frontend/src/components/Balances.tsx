import type { Balances } from '../types';

export default function BalancesPanel({ balances }: { balances: Balances }) {
  const assets = [
    { symbol: 'USD', color: 'var(--bid-color)', icon: '$' },
    { symbol: 'BTC', color: 'var(--accent)', icon: '₿' }
  ];

  return (
    <div className="glass-panel">
      <h2 className="panel-title">Wallet Balances</h2>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        {assets.map((asset) => {
          const bal = balances[asset.symbol] || { available: '0.00', locked: '0.00' };
          return (
            <div key={asset.symbol} style={{
              padding: '1.25rem', 
              background: 'rgba(0,0,0,0.15)', 
              borderRadius: '12px', 
              borderLeft: `4px solid ${asset.color}`,
              boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.02)'
            }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '1rem' }}>
                <div style={{
                  width: '32px', height: '32px', borderRadius: '50%',
                  background: asset.color, color: '#000',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                  fontWeight: 'bold', fontSize: '1.2rem',
                  boxShadow: `0 0 15px ${asset.color}40`
                }}>
                  {asset.icon}
                </div>
                <div style={{ fontWeight: 'bold', fontSize: '1.25rem' }}>{asset.symbol}</div>
              </div>
              
              <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.95rem' }}>
                  <span className="text-muted">Available</span>
                  <span className="mono">{Number(bal.available).toFixed(4)}</span>
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.95rem' }}>
                  <span className="text-muted">Locked</span>
                  <span className="mono">{Number(bal.locked).toFixed(4)}</span>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
