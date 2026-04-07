-- 0003_add_prices_table

CREATE TABLE IF NOT EXISTS prices (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  base text NOT NULL,
  quote text NOT NULL,
  price numeric NOT NULL,
  fetched_at timestamptz NOT NULL,
  source text,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_prices_base_quote ON prices (base, quote);
CREATE INDEX IF NOT EXISTS idx_prices_fetched_at ON prices (fetched_at);
