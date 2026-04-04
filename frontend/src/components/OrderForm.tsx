import React, { useState } from 'react';

export default function OrderForm({ userId, onOrderPlaced }: { userId: string, onOrderPlaced: () => void }) {
  const [type, setType] = useState('limit');
  const [side, setSide] = useState<'Bid' | 'Ask'>('Bid');
  const [price, setPrice] = useState('');
  const [size, setSize] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!size || (type === 'limit' && !price)) return;
    
    setLoading(true);
    try {
      const res = await fetch('/api/v1/orders', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          user_id: userId,
          base: 'BTC',
          quote: 'USD',
          price: type === 'limit' ? Number(price) : 0,
          size: Number(size),
          bid_or_ask: side,
          order_type: type,
        })
      });
      if (res.ok) {
        setPrice('');
        setSize('');
        onOrderPlaced();
      } else {
        const err = await res.text();
        alert('Order failed: ' + err);
      }
    } catch (e) {
      console.error(e);
    }
    setLoading(false);
  };

  return (
    <div className="glass-panel">
      <h2 className="panel-title">Trade Action</h2>
      
      <div style={{ display: 'flex', gap: '0.75rem', marginBottom: '1.5rem', background: 'rgba(0,0,0,0.2)', padding: '0.5rem', borderRadius: '14px' }}>
        <button 
          type="button"
          className={`btn ${side === 'Bid' ? 'btn-bid' : ''}`} 
          style={{ flex: 1, background: side === 'Bid' ? 'var(--bid-color)' : 'transparent', color: side === 'Bid' ? '#000' : 'var(--text-main)', boxShadow: side === 'Bid' ? '' : 'none' }}
          onClick={() => setSide('Bid')}
        >Buy</button>
        <button 
          type="button"
          className={`btn ${side === 'Ask' ? 'btn-ask' : ''}`} 
          style={{ flex: 1, background: side === 'Ask' ? 'var(--ask-color)' : 'transparent', color: side === 'Ask' ? '#fff' : 'var(--text-main)', boxShadow: side === 'Ask' ? '' : 'none' }}
          onClick={() => setSide('Ask')}
        >Sell</button>
      </div>

      <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1.5rem' }}>
        <div style={{ flex: 1, textAlign: 'center', padding: '0.75rem', cursor: 'pointer', borderBottom: type === 'limit' ? '2px solid var(--accent)' : '2px solid transparent', color: type === 'limit' ? 'var(--text-main)' : 'var(--text-muted)', fontWeight: type === 'limit' ? 600 : 400, transition: 'all 0.2s' }} onClick={() => setType('limit')}>Limit</div>
        <div style={{ flex: 1, textAlign: 'center', padding: '0.75rem', cursor: 'pointer', borderBottom: type === 'market' ? '2px solid var(--accent)' : '2px solid transparent', color: type === 'market' ? 'var(--text-main)' : 'var(--text-muted)', fontWeight: type === 'market' ? 600 : 400, transition: 'all 0.2s' }} onClick={() => setType('market')}>Market</div>
      </div>

      <form onSubmit={handleSubmit}>
        {type === 'limit' && (
          <div className="form-group">
            <label>Price (USD)</label>
            <input type="number" step="0.01" value={price} onChange={e => setPrice(e.target.value)} required placeholder="0.00" />
          </div>
        )}
        <div className="form-group">
          <label>Amount (BTC)</label>
          <input type="number" step="0.0001" value={size} onChange={e => setSize(e.target.value)} required placeholder="0.0000" />
        </div>
        
        {type === 'limit' && price && size && (
          <div style={{ marginBottom: '1.5rem', fontSize: '0.95rem', display: 'flex', justifyContent: 'space-between', background: 'rgba(0,0,0,0.15)', padding: '1rem', borderRadius: '8px' }}>
            <span className="text-muted">Total Value</span>
            <span className="mono">{(Number(price) * Number(size)).toFixed(2)} USD</span>
          </div>
        )}

        <button type="submit" className={`btn ${side === 'Bid' ? 'btn-bid' : 'btn-ask'}`} style={{ width: '100%', marginTop: '0.5rem', padding: '1rem' }} disabled={loading}>
          {loading ? 'Processing...' : `Place ${side === 'Bid' ? 'Buy' : 'Sell'} Order`}
        </button>
      </form>
    </div>
  );
}
