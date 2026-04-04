export interface Balance {
  available: string | number;
  locked: string | number;
}

export type Balances = Record<string, Balance>;

export interface Trade {
  id: string;
  price: string | number;
  quantity: string | number;
  buyer_id: string;
  seller_id: string;
  buy_order_id?: string;
  sell_order_id?: string;
}

export interface Order {
  id: string;
  user_id: string;
  size: string | number;
  bid_or_ask: 'Bid' | 'Ask';
  time_in_force: 'GTC' | 'IOC' | 'FOK';
}

export interface WsMessage {
  Trade?: Trade;
  OrderPlaced?: Order;
  OrderCancelled?: { order_id: string };
}
