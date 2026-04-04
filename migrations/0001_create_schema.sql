




BEGIN;



CREATE EXTENSION IF NOT EXISTS pgcrypto;



CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    username TEXT NOT NULL UNIQUE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);



CREATE TABLE IF NOT EXISTS balances (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    available NUMERIC(30,10) NOT NULL DEFAULT 0,
    locked NUMERIC(30,10) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, asset)
);



CREATE TABLE IF NOT EXISTS orders (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    side TEXT NOT NULL CHECK (side IN ('buy','sell')),
    order_type TEXT NOT NULL CHECK (order_type IN ('limit','market','fok','stop')),
    status TEXT NOT NULL CHECK (status IN ('open','partially_filled','filled','cancelled')),
    price NUMERIC(30,10), 

    amount NUMERIC(30,10) NOT NULL,
    filled NUMERIC(30,10) NOT NULL DEFAULT 0,
    client_order_id TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_price ON orders(price);



CREATE TABLE IF NOT EXISTS trades (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    buy_order_id uuid REFERENCES orders(id) ON DELETE SET NULL,
    sell_order_id uuid REFERENCES orders(id) ON DELETE SET NULL,
    price NUMERIC(30,10) NOT NULL,
    amount NUMERIC(30,10) NOT NULL,
    maker_order_id uuid,
    metadata JSONB,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_trades_executed_at ON trades(executed_at);



CREATE TABLE IF NOT EXISTS ledger (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    asset TEXT NOT NULL,
    delta NUMERIC(30,10) NOT NULL,
    balance_before NUMERIC(30,10),
    balance_after NUMERIC(30,10),
    reason TEXT,
    related_order uuid REFERENCES orders(id) ON DELETE SET NULL,
    related_trade uuid REFERENCES trades(id) ON DELETE SET NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ledger_user_time ON ledger(user_id, created_at);

COMMIT;
