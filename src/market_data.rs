pub enum MarketData {
    PositionUpdate(Position),
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub avg_price: f64,
    pub current_price: f64,
    pub profit_loss: f64,
}