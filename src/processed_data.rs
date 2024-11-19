use std::collections::HashMap;

pub enum Side {
    Buy,
    Sell,
}

pub struct OrderBookRow {
    pub price: f64,
    pub side: Side,
    pub quantity: i32,
    pub position: i32,
    pub message_number: i32,
}

pub struct OrderBookBlock {
    ticker: String,
    pub buy_rows: Vec<OrderBookRow>,
    pub sell_rows: Vec<OrderBookRow>,
}

pub struct OrderBook {
    pub id: String,
    pub order_book: HashMap<String, OrderBookBlock>,
}
impl OrderBook {
    pub fn new(id: &str) -> Self {
        OrderBook {
            id: id.to_string(),
            order_book: HashMap::new(),
        }
    }
    pub fn add_row(&mut self, ticker: &str, row: OrderBookRow) {
        let order_book_block = self.order_book.entry(ticker.to_string())
            .or_insert_with(|| OrderBookBlock {
                ticker: ticker.to_string(),
                buy_rows: Vec::new(),
                sell_rows: Vec::new(),
            });
        match row.side {
            Side::Buy => order_book_block.buy_rows.push(row),
            Side::Sell => order_book_block.sell_rows.push(row),
        }
    }
    pub fn remove_row(&mut self, price: f64) {
        self.order_book.values_mut().for_each(|block| {
            block.buy_rows.retain(|row| row.price != price);
            block.sell_rows.retain(|row| row.price != price);
        });
    }
    pub fn update_row(&mut self, price: f64, quantity: i32, message_number: i32) {
        for block in self.order_book.values_mut() {
            for row in block.buy_rows.iter_mut().chain(block.sell_rows.iter_mut()) {
                if row.price == price {
                    row.quantity = quantity;
                    row.message_number = message_number;
                }
            }
        }
    }
    fn get_orders_for_ticker(&self, ticker: &str) -> Option<&OrderBookBlock> {
        self.order_book.get(ticker)
    }
}

pub struct QuoteData {
    pub ticker: Option<String>,
    pub ask_price: Option<f64>,
    pub bid_price: Option<f64>,
    pub last_trade: Option<f64>,
    pub last_trade_time: Option<String>,
}
pub struct QuoteBook {
    pub id: String,
    pub quotes_list: Vec<QuoteData>,
}
impl QuoteBook {
    pub fn new(id: &str) -> Self {
        QuoteBook {
            id: id.to_string(),
            quotes_list: Vec::new(),
        }
    }
    pub fn add_quote(&mut self, quote_data: QuoteData) {
        if let Some(existing_quote) = self.quotes_list.iter_mut().find(|quote| quote.ticker == quote_data.ticker) {
            if let Some(ask_price) = &quote_data.ask_price {
               existing_quote.ask_price = Some(ask_price.clone()); 
            }
            if let Some(bid_price) = &quote_data.bid_price {
                existing_quote.bid_price = Some(bid_price.clone());
            }
            if let Some(last_trade) = &quote_data.last_trade {
                existing_quote.last_trade = Some(last_trade.clone());
            }
            if let Some(last_trade_time) = &quote_data.last_trade_time {
                existing_quote.last_trade_time = Some(last_trade_time.clone());
            }
        } else {
            self.quotes_list.push(quote_data);
        }        
    }
}

pub struct Position {
    position_id: u64,
    i: String,
    q: u64,
    open_price: f64,
    current_price: f64,
    pnl_price: f64,
}
pub struct Portfolio {
    id: String,
    portfolio: Vec<Position>,
}
impl Portfolio {
    pub fn new(id: &str) -> Self {
        Portfolio {
            id: id.to_string(),
            portfolio: Vec::new(),
        }
    }
}