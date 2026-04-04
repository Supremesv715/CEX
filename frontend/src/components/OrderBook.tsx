interface OrderBookProps {
  bids: [number, number][]; 

  asks: [number, number][]; 

}

export default function OrderBook({ bids, asks }: OrderBookProps) {
  const maxVol = Math.max(
    ...bids.map(b => b[1]),
    ...asks.map(a => a[1]),
    0.01 
  );

  return (
    <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
      <h2 className="panel-title">Order Book <span className="text-muted" style={{ fontSize: '0.85rem', fontWeight: 400 }}>(BTC/USD)</span></h2>
      
      <div className="orderbook-header">
        <span>Price(USD)</span>
        <span style={{ textAlign: 'right' }}>Size(BTC)</span>
        <span style={{ textAlign: 'right' }}>Total(USD)</span>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', flexDirection: 'column-reverse', flex: 1, justifyContent: 'flex-end' }}>
          {asks.slice(0, 16).map(([price, size], i) => (
            <div key={`ask-${price}-${i}`} className="orderbook-row mono">
              <div className="depth-bar ask" style={{ width: `${(size / maxVol) * 100}%` }}></div>
              <span className="text-ask">{Number(price).toFixed(2)}</span>
              <span>{Number(size).toFixed(4)}</span>
              <span className="text-muted">{(Number(price) * Number(size)).toFixed(2)}</span>
            </div>
          ))}
        </div>

        <div style={{ padding: '1rem 0', margin: '0.5rem 0', textAlign: 'center', fontSize: '1.25rem', fontWeight: 'bold', background: 'rgba(0,0,0,0.1)', borderRadius: '8px' }}>
          {asks.length > 0 && bids.length > 0 ? (
            <span style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '1rem' }}>
              <span className="text-main mono">${asks[0][0].toFixed(2)}</span>
              <span style={{ fontSize: '0.85rem', color: 'var(--text-muted)', fontWeight: 400 }}>Spread: ${(asks[0][0] - bids[0][0]).toFixed(2)}</span>
            </span>
          ) : <span className="text-muted">Waiting for orders...</span>}
        </div>

        <div style={{ flex: 1 }}>
          {bids.slice(0, 16).map(([price, size], i) => (
            <div key={`bid-${price}-${i}`} className="orderbook-row mono">
              <div className="depth-bar bid" style={{ width: `${(size / maxVol) * 100}%` }}></div>
              <span className="text-bid">{Number(price).toFixed(2)}</span>
              <span>{Number(size).toFixed(4)}</span>
              <span className="text-muted">{(Number(price) * Number(size)).toFixed(2)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
