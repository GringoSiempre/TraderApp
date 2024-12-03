use crate::processed_data::Position;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SLType {
    None,
    LossLimiter,
    BreakEven,
    TrailingStop,
}
impl SLType {
    pub fn description(&self) -> &str {
        match self {
            SLType::None => "none",
            SLType::LossLimiter => "Loss limiter",
            SLType::BreakEven => "Break-even",
            SLType::TrailingStop => "Trailing stop",
        }
    }
}

pub fn check_sl(position: &Position) -> (SLType, f64, bool) {
    let mut sl_type = position.sl_type;
    let mut sl_price = position.sl_price;
    let mut close_alert = position.close_alert;

    if position.current_price <= sl_price {
        close_alert = true;
    } else {
        match sl_type {
            SLType::None => {
                sl_type = SLType::LossLimiter;
                sl_price = position.current_price - 0.1;
            },
            SLType::LossLimiter => {
                if (position.current_price - position.open_price) >= 0.12 {
                    sl_type = SLType::BreakEven;
                    sl_price = position.open_price + 0.02;
                }
            },
            SLType::BreakEven => {
                if (position.current_price - position.open_price) >= 0.2 {
                    sl_type = SLType::TrailingStop;
                    sl_price = position.open_price + position.pnl / position.quantity as f64 / 2.0;
                }
            },
            SLType::TrailingStop => {
                let new_sl_price = position.open_price + position.pnl / position.quantity as f64 / 2.0;
                if new_sl_price > sl_price { sl_price = new_sl_price; }
            },
        }
    }
    (sl_type, sl_price, close_alert)
}