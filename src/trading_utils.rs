use chrono::{Datelike, Local, Weekday, Duration};
use crate::processed_data::{Side, Position};

#[derive(Debug, Clone, PartialEq)]
pub struct TickerOptions {
    pub ticker: String,
    pub short_option: String,
    pub long_option: String,
}
impl TickerOptions {
    pub fn new(ticker: String) -> Self {
        TickerOptions {
            ticker,
            short_option: "".to_string(),
            long_option: "".to_string(),
        }
    }
    pub fn update(&mut self, side: Side, current_price: f64, days_to_expiration: i64) {
        if current_price != 0.0 {
            let mut ticker = self.ticker.clone();
                ticker.truncate(self.ticker.len() - 3); // Обрезаем .US в конце тикера
            let today = Local::now().naive_local().date();
            let mut future_date = today + Duration::days(days_to_expiration);
            // Если дата опционов выпадает на субботу или воскресенье, сдвигаем ее на понедельник
            if future_date.weekday() == Weekday::Sat { future_date = future_date + Duration::days(2); }
            else if future_date.weekday() == Weekday::Sun { future_date = future_date + Duration::days(2); }
            let day = format!("{:02}", future_date.day()); // Извлекаем день из даты опциона
            let month = format!("{}", future_date.format("%b").to_string().to_uppercase()); // Извлекаем месяц из даты опциона в формате МММ
            let year = format!("{:04}", future_date.year()); // Извлекаем день из даты опциона
            match side {
                Side::Buy => {
                    let put_strike = current_price.floor() as i32;
                    let short_option_ticker = format!("+{}.{}{}{}.P{}", ticker, day, month, year, put_strike);
                    self.short_option = short_option_ticker;
                }
                Side::Sell => {
                    let call_strike = current_price.ceil() as i32;
                    let long_option_ticker = format!("+{}.{}{}{}.C{}", ticker, day, month, year, call_strike);
                    self.long_option = long_option_ticker;
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SLStrategy {
    WithoutStops,
    InsuranceStops,
    ManualStops,
    CandleTailsStops,
}
impl SLStrategy {
    pub fn description(&self) -> &str {
        match self {
            SLStrategy::WithoutStops => "w/o stops",
            SLStrategy::InsuranceStops => "insurance",
            SLStrategy::ManualStops => "manual",
            SLStrategy::CandleTailsStops => "candle tails",
        }
    }
    pub const ALL: [SLStrategy; 4] = [
        SLStrategy::WithoutStops,
        SLStrategy::InsuranceStops,
        SLStrategy::ManualStops,
        SLStrategy::CandleTailsStops,
    ];
}

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
            SLType::LossLimiter => "loss limiter",
            SLType::BreakEven => "break-even",
            SLType::TrailingStop => "trailing stop",
        }
    }
}

pub fn check_sl(position: &Position) -> (SLType, f64, bool) {
    let sl_strategy = position.sl_strategy;
    let mut sl_type = position.sl_type;
    let mut sl_price = position.sl_price;
    let mut close_alert = position.close_alert;

    if position.current_price <= sl_price {
        close_alert = true;
    } else {
        match sl_strategy {
            SLStrategy::InsuranceStops => {
                match sl_type {
                    SLType::None => {
                        sl_type = SLType::LossLimiter;
                        sl_price = f64::min(position.current_price, position.open_price) - 0.1;
                    },
                    SLType::LossLimiter => {
                        if (position.current_price - position.open_price) >= 0.11 {
                            sl_type = SLType::BreakEven;
                            sl_price = position.open_price + 0.02;
                        } else if (position.current_price - position.sl_price) > 0.10 {
                            sl_price = position.current_price - 0.1;
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
            _ => {}
        }
    }
    (sl_type, sl_price, close_alert)
}
pub fn upgrade_sl(position: &Position) -> (SLType, f64) {
    let sl_strategy = position.sl_strategy;
    let mut sl_type = position.sl_type;
    let mut sl_price = position.sl_price;

    match sl_strategy {
        SLStrategy::InsuranceStops => {
            match sl_type {
                SLType::None => {
                    sl_type = SLType::LossLimiter;
                    sl_price = f64::min(position.current_price, position.open_price) - 0.1;
                },
                SLType::LossLimiter => {
                    sl_type = SLType::BreakEven;
                    sl_price = position.open_price + 0.02;
                },
                SLType::BreakEven => {
                    if position.pnl >= 0.02 {
                        sl_type = SLType::TrailingStop;
                        sl_price = position.open_price + position.pnl / position.quantity as f64 / 2.0;
                    }
                },
                SLType::TrailingStop => {
                },
            }
        }
        _ => {}
    }

    (sl_type, sl_price)
}