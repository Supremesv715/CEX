import { useEffect, useState } from 'react';
import BalancesPanel from './components/Balances';
import OrderForm from './components/OrderForm';
import OrderBook from './components/OrderBook';
import RecentTrades from './components/RecentTrades';
import type { Balances, Trade, WsMessage } from './types';

function App() {
  const [userId, setUserId] = useState<string | null>(null);
  const [balances, setBalances] = useState<Balances>({});
  const [trades, setTrades] = useState<Trade[]>([]);
  const [bids, setBids] = useState<[number, number][]>([]);
  const [asks, setAsks] = useState<[number, number][]>([]);
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    const initUser = async () => {
      let stored = null;
      if (!stored) {
        try {
          const res = await fetch('/api/v1/users', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ initial_funds: 100000.0 })
          });
          const data = await res.json();
          stored = data.user_id;
          localStorage.setItem('exchange_user_id', stored as string);
        } catch (e) {
          console.error('API not reachable:', e);
          return;
        }
      }
      setUserId(stored);
    };
    initUser();
  }, []);

  const fetchBalances = async () => {
    if (!userId) return;
    try {
      const res = await fetch(`/api/v1/users/${userId}/balances`);
      if (res.ok) setBalances(await res.json());
    } catch {}
  };

  const fetchOrderBook = async () => {
    try {
      const res = await fetch('/api/v1/market/BTC/USD/orderbook');
      if (res.ok) {
          const data = await res.json();
          setBids(data.bids ? data.bids.map((x: any) => [Number(x[0]), Number(x[1])]) : []);
          setAsks(data.asks ? data.asks.map((x: any) => [Number(x[0]), Number(x[1])]) : []);
      }
    } catch {}
  };

  useEffect(() => {
    if (userId) {
      fetchBalances();
      fetchOrderBook();
      
      let wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      let wsHost = window.location.host;
      const ws = new WebSocket(`${wsProtocol}//${wsHost}/api/v1/ws`);
      
      ws.onopen = () => setIsConnected(true);
      ws.onclose = () => setIsConnected(false);

      ws.onmessage = (event) => {
        const msg: WsMessage = JSON.parse(event.data);
        if (msg.Trade) {
          setTrades(prev => [msg.Trade!, ...prev].slice(0, 50));
          fetchBalances();
          fetchOrderBook();
        } else if (msg.OrderPlaced || msg.OrderCancelled) {
          fetchBalances();
          fetchOrderBook();
        }
      };
      
      return () => {
        ws.close();
      };
    }
  }, [userId]);

  if (!userId) {
    return (
      <div style={{ display: 'flex', height: '100vh', justifyContent: 'center', alignItems: 'center' }}>
        <div className="glass-panel pulse" style={{ padding: '3rem 5rem', fontSize: '1.5rem', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem' }}>
          <div className="brand">
            <div className="brand-icon">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{color: 'white'}}><polygon points="12 2 2 22 22 22"></polygon></svg>
            </div>
            Aura
          </div>
          <div>Connecting to Engine...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="app-container">
      <header className="header">
        <div className="brand">
          <div className="brand-icon">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{color: 'white'}}><polygon points="12 2 2 22 22 22"></polygon></svg>
          </div>
          Aura Exchange
        </div>
        <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: '20px' }}>
            <div style={{ 
              width: '10px', height: '10px', borderRadius: '50%', 
              background: isConnected ? 'var(--bid-color)' : 'var(--ask-color)', 
              boxShadow: `0 0 10px ${isConnected ? 'var(--bid-color)' : 'var(--ask-color)'}` 
            }}></div>
            <span className="text-muted" style={{ fontSize: '0.85rem' }}>{isConnected ? 'API Connected' : 'Disconnected'}</span>
          </div>
          <div className="text-muted" style={{ fontSize: '0.85rem', background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: '20px'  }}>
            UID: <span className="mono text-main">{userId.substring(0, 8)}</span>
          </div>
        </div>
      </header>

      <aside className="left-sidebar">
        <BalancesPanel balances={balances} />
        <OrderForm userId={userId} onOrderPlaced={() => { fetchBalances(); fetchOrderBook(); }} />
      </aside>

      <main className="main-content">
        <OrderBook bids={bids} asks={asks} />
      </main>

      <aside className="right-sidebar">
        <RecentTrades trades={trades} />
      </aside>
    </div>
  );
}

export default App;
