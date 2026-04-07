import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { PriceInfo } from '../types';

const MAX_POINTS = 400;

interface Props {
  base: string;
  quote: string;
  livePrice: PriceInfo | null;
}

type Pt = { t: number; label: string; price: number };

function pairMatches(live: PriceInfo, base: string, quote: string) {
  return live.base.toUpperCase() === base.toUpperCase() && live.quote.toUpperCase() === quote.toUpperCase();
}

export default function LivePriceChart({ base, quote, livePrice }: Props) {
  const [data, setData] = useState<Pt[]>([]);
  const pairKey = `${base.toUpperCase()}/${quote.toUpperCase()}`;
  const lastLiveRef = useRef<string | null>(null);

  const appendSnapshot = useCallback((snap: PriceInfo) => {
    if (!pairMatches(snap, base, quote)) return;
    const fa = snap.fetched_at as string | undefined;
    const ts = fa ? Date.parse(fa) : Date.now();
    const price = Number(snap.price);
    if (Number.isNaN(price) || Number.isNaN(ts)) return;
    const key = `${fa ?? ts}:${price}`;
    if (lastLiveRef.current === key) return;
    lastLiveRef.current = key;
    const label = new Date(ts).toLocaleTimeString();
    setData((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.t === ts) {
        next[next.length - 1] = { t: ts, label, price };
        return next;
      }
      if (last && ts < last.t) return prev;
      next.push({ t: ts, label, price });
      if (next.length > MAX_POINTS) next.splice(0, next.length - MAX_POINTS);
      return next;
    });
  }, [base, quote]);

  useEffect(() => {
    let cancelled = false;
    lastLiveRef.current = null;

    const loadHistory = async () => {
      try {
        const res = await fetch(
          `/api/v1/price/${encodeURIComponent(base)}/${encodeURIComponent(quote)}/history?limit=300`
        );
        if (!res.ok || cancelled) return;
        const rows: { fetched_at: string; price: string }[] = await res.json();
        const mapped = rows
          .map((r) => ({
            t: Date.parse(r.fetched_at),
            label: new Date(r.fetched_at).toLocaleTimeString(),
            price: Number(r.price),
          }))
          .filter((p) => !Number.isNaN(p.price) && !Number.isNaN(p.t));
        if (!cancelled) {
          setData((prev) => {
            if (mapped.length === 0) return prev;
            return mapped;
          });
        }
      } catch {
        /* ignore */
      }
    };

    void loadHistory();
    const interval = window.setInterval(loadHistory, 20_000);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [base, quote]);

  useEffect(() => {
    if (!livePrice) return;
    appendSnapshot(livePrice);
  }, [livePrice, appendSnapshot]);

  useEffect(() => {
    let cancelled = false;
    const poll = async () => {
      try {
        const res = await fetch(
          `/api/v1/price/${encodeURIComponent(base)}/${encodeURIComponent(quote)}`
        );
        if (!res.ok || cancelled) return;
        appendSnapshot((await res.json()) as PriceInfo);
      } catch {
        /* ignore */
      }
    };
    void poll();
    const id = window.setInterval(poll, 6000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [base, quote, appendSnapshot]);

  const chartData = useMemo(() => {
    if (data.length >= 2) return data;
    if (data.length === 1) {
      const a = data[0];
      return [a, { ...a, t: a.t + 1, label: a.label }];
    }
    return [];
  }, [data]);

  const paths = useMemo(() => {
    if (chartData.length < 2) return null;
    const w = 800;
    const h = 240;
    const pad = 28;
    const ts = chartData.map((d) => d.t);
    const prices = chartData.map((d) => d.price);
    const minT = Math.min(...ts);
    const maxT = Math.max(...ts);
    const minP = Math.min(...prices);
    const maxP = Math.max(...prices);
    const dT = maxT - minT || 1;
    const dP = maxP - minP || 1e-9;
    const x0 = pad;
    const x1 = w - pad;
    const y0 = pad;
    const y1 = h - pad;
    const X = (t: number) => x0 + ((t - minT) / dT) * (x1 - x0);
    const Y = (p: number) => y1 - ((p - minP) / dP) * (y1 - y0);
    let line = '';
    chartData.forEach((d, i) => {
      const x = X(d.t);
      const y = Y(d.price);
      line += i === 0 ? `M ${x} ${y}` : ` L ${x} ${y}`;
    });
    const lx = X(chartData[chartData.length - 1].t);
    const fx = X(chartData[0].t);
    const area = `${line} L ${lx} ${y1} L ${fx} ${y1} Z`;
    const last = data[data.length - 1];
    return { line, area, w, h, minP, maxP, last };
  }, [chartData, data]);

  if (data.length === 0) {
    return (
      <div className="glass-panel">
        <h2 className="panel-title">Live price</h2>
        <p className="text-muted" style={{ fontSize: '0.9rem', lineHeight: 1.5 }}>
          Waiting for a live <span className="mono">{pairKey}</span> price from the engine. The server polls every ~15s when it can reach CoinGecko and stores
          rows in <span className="mono">prices</span>. This panel also polls <span className="mono">GET /api/v1/price/BTC/USD</span>{' '}
          every 6s so the chart fills even if WebSocket delivery misses updates.
        </p>
        <p className="text-muted" style={{ fontSize: '0.85rem', lineHeight: 1.5, marginTop: '0.75rem' }}>
          Check the terminal running <span className="mono">cargo run</span>: you should see errors if CoinGecko is blocked,
          DNS fails, or database inserts fail. If <span className="mono">price snapshot DB insert failed</span> appears, verify
          migration <span className="mono">0003_add_prices_table.sql</span> applied.
        </p>
      </div>
    );
  }

  if (!paths) return null;

  return (
    <div className="glass-panel">
      <h2 className="panel-title">
        Live price · {pairKey}{' '}
        <span className="mono text-bid" style={{ fontSize: '1rem', fontWeight: 700 }}>
          {paths.last.price.toFixed(2)}
        </span>
      </h2>
      <svg
        viewBox={`0 0 ${paths.w} ${paths.h}`}
        preserveAspectRatio="xMidYMid meet"
        role="img"
        aria-label={`Price chart for ${pairKey}`}
        style={{ width: '100%', height: 'auto', display: 'block', maxHeight: 320 }}
      >
        <defs>
          <linearGradient id="livePriceFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--bid-color)" stopOpacity={0.35} />
            <stop offset="100%" stopColor="var(--bid-color)" stopOpacity={0} />
          </linearGradient>
        </defs>
        <path d={paths.area} fill="url(#livePriceFill)" />
        <path
          d={paths.line}
          fill="none"
          stroke="var(--bid-color)"
          strokeWidth={2.5}
          strokeLinejoin="round"
          strokeLinecap="round"
        />
      </svg>
      <div
        className="text-muted"
        style={{ fontSize: '0.75rem', marginTop: '0.5rem', display: 'flex', justifyContent: 'space-between', gap: '1rem', flexWrap: 'wrap' }}
      >
        <span>
          Low {paths.minP.toFixed(2)} · High {paths.maxP.toFixed(2)}
        </span>
        <span className="mono">{data.length} samples</span>
      </div>
    </div>
  );
}
