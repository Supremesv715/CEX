
import type { OpenOrder } from '../types';

interface OpenOrdersProps {
  orders: OpenOrder[];
  onCancel: (id: string) => void;
}

export default function OpenOrders({ orders, onCancel }: OpenOrdersProps) {
  return (
    <div className="glass-panel" style={{ marginTop: '1.5rem', flex: 1, minHeight: '300px' }}>
      <h2 className="panel-title">Open Orders</h2>
      <div className="table-responsive">
        <table style={{ width: '100%', textAlign: 'left', borderCollapse: 'collapse' }}>
          <thead>
            <tr>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Market</th>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Side</th>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Type</th>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Price</th>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Amount</th>
              <th className="text-muted" style={{ paddingBottom: '0.5rem', borderBottom: '1px solid rgba(255,255,255,0.1)' }}>Action</th>
            </tr>
          </thead>
          <tbody>
            {orders.length === 0 ? (
              <tr>
                <td colSpan={6} style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-muted)' }}>
                  No open orders
                </td>
              </tr>
            ) : (
              orders.map(order => (
                <tr key={order.id} className="animate-fade-in" style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                  <td style={{ paddingTop: '0.75rem', paddingBottom: '0.75rem' }}>{order.market ? order.market.replace('_', '/') : 'BTC/USD'}</td>
                  <td className={order.side === 'buy' ? 'text-bid' : 'text-ask'} style={{ textTransform: 'capitalize' }}>
                    {order.side}
                  </td>
                  <td style={{ textTransform: 'capitalize' }}>{order.order_type}</td>
                  <td className="mono">{order.price ? Number(order.price).toFixed(2) : '-'}</td>
                  <td className="mono">{Number(order.amount).toFixed(4)}</td>
                  <td>
                    <button 
                      onClick={() => onCancel(order.id)} 
                      style={{ 
                        background: 'rgba(255, 60, 60, 0.2)', 
                        color: '#ff6b6b', 
                        border: '1px solid rgba(255, 60, 60, 0.4)', 
                        padding: '0.2rem 0.6rem', 
                        borderRadius: '4px', 
                        cursor: 'pointer',
                        fontSize: '0.75rem',
                        transition: 'all 0.2s'
                      }}
                      onMouseOver={(e) => { e.currentTarget.style.background = 'rgba(255, 60, 60, 0.4)' }}
                      onMouseOut={(e) => { e.currentTarget.style.background = 'rgba(255, 60, 60, 0.2)' }}
                    >
                      Cancel
                    </button>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
