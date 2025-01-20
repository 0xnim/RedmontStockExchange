use sqlx::types::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

// Enums for restricted string values
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Added type_name for SQL compatibility
pub enum InstrumentType {
    STOCK,
    ETF,
    BOND,
    COMMODITY,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from order_type to varchar to match SQL
pub enum OrderType {
    LIMIT,
    MARKET,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from order_side to varchar to match SQL
pub enum OrderSide {
    BUY,
    SELL,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from instrument_status to varchar to match SQL
pub enum InstrumentStatus {
    ACTIVE,
    SUSPENDED,
    DELISTED,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from broker_status to varchar to match SQL
pub enum BrokerStatus {
    ACTIVE,
    SUSPENDED,
    TERMINATED,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from position_status to varchar to match SQL
pub enum OrderStatus {
    PENDING,
    PARTIAL,
    FILLED,
    CANCELLED,
    REJECTED,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[sqlx(type_name = "varchar")] // Changed from trade_status to varchar to match SQL
pub enum TradeStatus {
    PENDING_SETTLEMENT,
    SETTLED,
    FAILED,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Instrument {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: InstrumentType,
    pub status: InstrumentStatus,
    pub lot_size: i32,
    pub tick_size: Decimal, // Changed from f64 to Decimal to match SQL DECIMAL(10,4)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Broker {
    pub id: Uuid,
    pub broker_code: String,
    pub name: String,
    pub status: BrokerStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct CashPosition {
    pub id: Uuid,
    pub broker_id: Uuid,
    pub currency: String,
    pub total_balance: Decimal, // Changed from f64 to Decimal to match SQL DECIMAL(20,4)
    pub locked_balance: Decimal, // Changed from f64 to Decimal to match SQL DECIMAL(20,4)
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct SecurityPosition {
    pub id: Uuid,
    pub broker_id: Uuid,
    pub instrument_id: Uuid,
    pub total_quantity: Decimal,
    pub locked_quantity: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Order {
    pub id: Uuid,
    pub broker_id: Uuid,
    pub instrument_id: Uuid,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub status: OrderStatus,
    pub price: Option<Decimal>,
    pub original_quantity: Decimal,
    pub remaining_quantity: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Trade {
    pub id: Uuid,
    pub instrument_id: Uuid,
    pub buyer_order_id: Uuid,
    pub seller_order_id: Uuid,
    pub buyer_broker_id: Uuid,
    pub seller_broker_id: Uuid,
    pub price: Decimal,
    pub quantity: Decimal,
    pub execution_time: DateTime<Utc>,
    pub status: TradeStatus,
    pub settlement_time: Option<DateTime<Utc>>,
}

// These index structs appear to be helpers for database queries
// They match the indices defined in SQL
#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct OrderIndex {
    pub instrument_id: Uuid,
    pub status: OrderStatus,
    pub price: Decimal,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct TradeIndex {
    pub status: TradeStatus,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct SecurityPositionIndex {
    pub broker_id: Uuid,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct CashPositionIndex {
    pub broker_id: Uuid,
}