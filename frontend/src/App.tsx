import { useEffect, useRef, useState } from 'react';
import BalancesPanel from './components/Balances';
import OrderForm from './components/OrderForm';
import OrderBook from './components/OrderBook';
import RecentTrades from './components/RecentTrades';
import PriceTicker from './components/PriceTicker';
import OpenOrders from './components/OpenOrders';
import LivePriceChart from './components/LivePriceChart';
import ChainSwitcher, { type WatchAsset } from './components/ChainSwitcher';
import type { Balances, Trade, WsMessage, PriceInfo, OpenOrder } from './types';

const QUOTE = 'USD';

function App() {
  const [userId, setUserId] = useState<string | null>(null);
  const [balances, setBalances] = useState<Balances>({});
  const [trades, setTrades] = useState<Trade[]>([]);
  const [bids, setBids] = useState<[number, number][]>([]);
  const [asks, setAsks] = useState<[number, number][]>([]);
  const [openOrders, setOpenOrders] = useState<OpenOrder[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [price, setPrice] = useState<PriceInfo | null>(null);
  const [watchBase, setWatchBase] = useState<WatchAsset>('BTC');
  const watchBaseRef = useRef<WatchAsset>(watchBase);
  watchBaseRef.current = watchBase;

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

  useEffect(() => {
    if (!userId) return;
    let cancelled = false;
    (async () => {
      try {
        const res = await fetch(`/api/v1/price/${watchBase}/${QUOTE}`);
        if (cancelled) return;
        if (res.ok) setPrice(await res.json());
        else setPrice(null);
      } catch {
        if (!cancelled) setPrice(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [userId, watchBase]);

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

  const fetchOpenOrders = async () => {
    if (!userId) return;
    try {
      const res = await fetch(`/api/v1/users/${userId}/orders`);
      if (res.ok) {
        setOpenOrders(await res.json());
      }
    } catch {}
  };

  const handleCancelOrder = async (orderId: string) => {
    try {
      const res = await fetch(`/api/v1/orders/${orderId}`, { method: 'DELETE' });
      if (res.ok) {
        fetchOpenOrders();
      } else {
        alert('Failed to cancel order: ' + (await res.text()));
      }
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    if (userId) {
      fetchBalances();
      fetchOrderBook();
      fetchOpenOrders();

      let wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      let wsHost = window.location.host;
      const ws = new WebSocket(`${wsProtocol}//${wsHost}/api/v1/ws`);

      ws.onopen = () => setIsConnected(true);
      ws.onclose = () => setIsConnected(false);

      ws.onmessage = (event) => {
        const raw = JSON.parse(event.data);
        if (raw.price !== undefined && raw.base && raw.quote) {
          const b = String(raw.base).toUpperCase();
          const q = String(raw.quote).toUpperCase();
          if (b === watchBaseRef.current && q === QUOTE) {
            setPrice(raw as PriceInfo);
          }
          return;
        }

        const msg: WsMessage = raw;
        if (msg.Trade) {
          setTrades(prev => [msg.Trade!, ...prev].slice(0, 50));
          fetchBalances();
          fetchOrderBook();
          fetchOpenOrders();
        } else if (msg.OrderPlaced || msg.OrderCancelled) {
          fetchBalances();
          fetchOrderBook();
          fetchOpenOrders();
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
        <div style={{ display: 'flex', gap: '1rem', alignItems: 'center', flexWrap: 'wrap', justifyContent: 'flex-end' }}>
          <ChainSwitcher value={watchBase} onChange={setWatchBase} />
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: '20px' }}>
            <div style={{
              width: '10px', height: '10px', borderRadius: '50%',
              background: isConnected ? 'var(--bid-color)' : 'var(--ask-color)',
              boxShadow: `0 0 10px ${isConnected ? 'var(--bid-color)' : 'var(--ask-color)'}`
            }}></div>
            <span className="text-muted" style={{ fontSize: '0.85rem' }}>{isConnected ? 'API Connected' : 'Disconnected'}</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center' }}>
            <PriceTicker price={price} />
          </div>
          <div className="text-muted" style={{ fontSize: '0.85rem', background: 'rgba(255,255,255,0.05)', padding: '0.5rem 1rem', borderRadius: '20px'  }}>
            UID: <span className="mono text-main">{userId.substring(0, 8)}</span>
          </div>
        </div>
      </header>

      <aside className="left-sidebar">
        <BalancesPanel balances={balances} />
        <OrderForm userId={userId} onOrderPlaced={() => { fetchBalances(); fetchOrderBook(); fetchOpenOrders(); }} />
      </aside>

      <main className="main-content" style={{ display: 'flex', flexDirection: 'column' }}>
        <OrderBook bids={bids} asks={asks} />
        <OpenOrders orders={openOrders} onCancel={handleCancelOrder} />
        <LivePriceChart base={watchBase} quote={QUOTE} livePrice={price} />
      </main>

      <aside className="right-sidebar">
        <RecentTrades trades={trades} />
      </aside>
    </div>
  );
}

export default App;
