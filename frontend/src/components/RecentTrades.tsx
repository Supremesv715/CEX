import type { Trade } from '../types';

export default function RecentTrades({ trades }: { trades: Trade[] }) {
  return (
    <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
      <h2 className="panel-title">Recent Trades <div className="pulse" style={{ width: '8px', height: '8px', borderRadius: '50%', background: 'var(--bid-color)', marginLeft: '10px' }}></div></h2>
      <div className="orderbook-header">
        <span>Price(USD)</span>
        <span>Size(BTC)</span>
        <span style={{ textAlign: 'right' }}>Time</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.25rem', overflowY: 'auto' }}>
        {trades.length === 0 && <div className="text-muted" style={{ padding: '2rem 0', textAlign: 'center' }}>No recent trades.</div>}
        {trades.map((t, i) => {
          

          

          

          const tradeColor = i % 2 === 0 ? "text-bid" : "text-ask"; 
          
          return (
             <div key={t.id + '-' + i} className="trade-row mono" style={{ animationDelay: `${Math.min(i * 0.05, 0.5)}s` }}>
               <span className={tradeColor}>{Number(t.price).toFixed(2)}</span>
               <span>{Number(t.quantity).toFixed(4)}</span>
               <span className="text-muted" style={{ fontSize: '0.8rem' }}>{new Date().toLocaleTimeString(undefined, {hour12: false, hour: '2-digit', minute:'2-digit', second:'2-digit'})}</span>
             </div>
          )
        })}
      </div>
    </div>
  );
}
