use serde::{Serialize, Deserialize};
use serde_json::*;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarketData {
    OrderBookMessage(OrderBookMessage),
    QuoteMessage(QuoteMessage),
    PortfolioMessage(PortfolioMessage),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteMessage {
    acd: Option<i32>, // Accumulated coupon interest (ACI)
    bac: Option<String>, // Best offer change mark (\'\'unchanged, \'D\'down, \'U\'up)
    baf: Option<i32>, // Volume of the best offer
    bap: Option<f64>, // Best offer
    bas: Option<i32>, // Value (size) of the best offer
    base_contract_code: Option<String>,
    base_currency: Option<String>,
    base_ltr: Option<String>,
    bat: Option<String>,
    bbc: Option<String>, // Designations of the best bid changes (\'\' – no changes, \'D\' - down, \'U\' - up)
    bbf: Option<i32>, // Best bid volume
    bbp: Option<f64>, // Best bid
    bbs: Option<i32>, // Best bid size
    bbt: Option<String>,
    c: Option<String>, // Ticker
    chg: Option<f64>, // Change in the price of the last trade in points, relative to the closing price of the previous trading session
    chg110: Option<f64>,
    chg22: Option<f64>,
    chg220: Option<f64>,
    chg5: Option<f64>,
    close_price: Option<f64>,
    codesub_nm: Option<String>,
    cpn: Option<i32>, // Coupon, in the currency
    cpp: Option<i32>, // Coupon period (in days)
    delta: Option<f64>,
    dpb: Option<i32>,
    dpd: Option<i32>, // Purchase margin
    dps: Option<i32>, // Short sale margin
    emitent_type: Option<String>,
    fv: Option<i32>, // Face value
    gamma: Option<f64>,
    init: Option<i32>,
    ipo: Option<String>,
    issue_nb: Option<String>,
    kind: Option<i32>,
    ltc: Option<String>, // Designations of price change (\'\' – no changes, \'D\' - down, \'U\' - up)
    ltp: Option<f64>, // Last trade price
    ltr: Option<String>, // Exchange of the latest trade
    lts: Option<i32>, // Last trade size
    ltt: Option<String>, // Time of last trade
    market_status: Option<String>,
    maxtp: Option<f64>, // Maximum trade price per day
    min_step: Option<f64>, // Minimum price increment
    mintp: Option<f64>, // Minimum trade price per day
    mrg: Option<String>,
    mtd: Option<String>, // Payment Date
    n: Option<i32>,
    name: Option<String>, // Name of security
    name2: Option<String>, // Security name in Latin
    ncd: Option<String>, // Next coupon date
    ncp: Option<i32>, // Latest coupon date
    op: Option<f64>, // Opening price of the current trading session
    option_type: Option<String>,
    otc_instr: Option<String>,
    p110: Option<f64>,
    p22: Option<f64>,
    p220: Option<f64>,
    p5: Option<f64>,
    pcp: Option<f64>, // Percentage change relative to the closing price of the previous trading session
    pp: Option<f64>, // Previous closing
    quote_basis: Option<String>,
    receptions: Option<String>,
    rev: Option<i64>,
    scheme_calc: Option<String>,
    step_price: Option<f64>, // Price increment
    strike_price: Option<f64>,
    theta: Option<f64>,
    trades: Option<i32>, // Number of trades
    trading_reference_price: Option<f64>,
    trading_session_sub_id: Option<String>,
    type_: Option<i32>,
    utc_offset: Option<i32>,
    virt_base_instr: Option<String>,
    vlt: Option<f64>, // Trading volume per day in currency
    vol: Option<i32>, // Trade volume per day, in pcs
    volatility: Option<f64>,
    x_agg_futures: Option<String>,
    x_curr: Option<String>,
    x_curr_val: Option<f64>,
    x_descr: Option<String>,
    x_dsc1: Option<i32>,
    x_dsc1_reception: Option<String>,
    x_dsc2: Option<i32>,
    x_dsc2_reception: Option<String>,
    x_dsc3: Option<i32>,
    x_istrade: Option<i32>,
    x_lot: Option<i32>,
    x_max: Option<f64>,
    x_min: Option<f64>,
    x_min_lot_q: Option<i32>,
    x_short: Option<i32>,
    x_short_reception: Option<String>,
    yld: Option<f64>, // Yield to maturity (for bonds)
    yld_ytm_ask: Option<f64>,
    yld_ytm_bid: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderBookMessage {
    n: i32,
    i: String,
    min_step: Option<f64>,
    step_price: Option<f64>,
    del: Vec<DeleteEntry>,
    ins: Vec<InsertEntry>,
    upd: Vec<UpdateEntry>,
    cnt: i32,
    x: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteEntry {
    p: f64,
    k: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct InsertEntry {
    p: f64,
    s: String,
    q: i32,
    k: i32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateEntry {
    p: f64,
    s: String,
    q: i32,
    k: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioMessage {
    loaded: bool,
    m_id: String,
    acc: Vec<AccountEntry>,
    pos: Vec<PositionEntry>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AccountEntry {
    s: f64, // Available funds
    k: i32,
    t: i32,    
    forecast_in: f64,
    forecast_out: f64,
    curr: String, // Account currency
    currval: f64, // Account currency exchange rate
    t2_in: f64,
    t2_out: f64,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct PositionEntry {
    i: String, // Open position ticker
    t: i32,
    k: i32,
    s: f64,
    q: i32, // Number of securities in the position
    fv: i32, // Coefficient to calculate initial margin
    curr: String, // Open position currency
    currval: f64, // Account currency exchange rate
    name: String, // Issuer name
    name2: String, // Issuer alternative name
    open_bal: f64, // Position book value
    mkt_price: f64, // Open position market value
    vm: String, // Variable margin of a position
    go: String, // Initial margin per position
    profit_close: f64, // Previous day positions profit
    acc_pos_id: i64, // Unique identifier of an open position in the Tradernet system
    accruedint_a: String, // (ACI) accrued coupon income
    acd: String,
    bal_price_a: f64, // Open position book value
    price_a: f64, // Book value of the position when opened
    base_currency: String,
    face_val_a: i32,
    scheme_calc: String,
    instr_id: i64,
    #[serde(rename = "Yield")]
    yield_: String,
    issue_nb: String,
    profit_price: f64, // Current position profit
    market_value: f64, // Asset value
    close_price: f64, // Position closing price
}

pub fn deserialize_message (message: &str) -> Option<MarketData> {
    let raw_values: Vec<Value> = serde_json::from_str(message).ok()?;
    let message_type = raw_values.get(0).and_then(|v| v.as_str())?;
    let data = raw_values.get(1)?;
    match message_type {
        "q" => {
            serde_json::from_value::<QuoteMessage>(data.clone())
                .ok()
                .map(MarketData::QuoteMessage)
        }
        "b" => {
            serde_json::from_value::<OrderBookMessage>(data.clone())
                .ok()
                .map(MarketData::OrderBookMessage)
        }
        "portfolio" => {
            serde_json::from_value::<PortfolioMessage>(data.clone())
                .map_err(|e| println!("Deserialization error: {:?}", e)).ok()
                // .ok()
                .map(MarketData::PortfolioMessage)
        }
        _ => None,
    }
}