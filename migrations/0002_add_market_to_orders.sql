

BEGIN;

ALTER TABLE orders
  ADD COLUMN IF NOT EXISTS market TEXT;

CREATE INDEX IF NOT EXISTS idx_orders_market ON orders(market);

COMMIT;
