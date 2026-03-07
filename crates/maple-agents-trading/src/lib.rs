//! MAPLE Reference Trading Agent — LLM decision loop for market operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide { Buy, Sell }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType { Market, Limit, StopLoss, StopLimit }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus { Pending, Submitted, PartialFill, Filled, Cancelled, Rejected }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradingStrategy { Momentum, MeanReversion, Arbitrage, MarketMaking, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: f64,
    pub price: Option<f64>,
    pub stop_price: Option<f64>,
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub filled_at: Option<DateTime<Utc>>,
    pub filled_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub avg_entry_price: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_order_value: f64,
    pub max_open_orders: usize,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self { max_position_size: 100000.0, max_daily_loss: 5000.0, max_order_value: 50000.0, max_open_orders: 10 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TradingError {
    #[error("risk limit exceeded: {0}")]
    RiskLimitExceeded(String),
    #[error("invalid order: {0}")]
    InvalidOrder(String),
    #[error("insufficient balance: need {needed}, have {available}")]
    InsufficientBalance { needed: f64, available: f64 },
    #[error("symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("order not found: {0}")]
    OrderNotFound(String),
}

pub struct TradingAgent {
    orders: Vec<TradeOrder>,
    positions: Vec<Position>,
    risk_limits: RiskLimits,
    balance: f64,
}

impl TradingAgent {
    pub fn new(balance: f64, risk_limits: RiskLimits) -> Self {
        Self { orders: Vec::new(), positions: Vec::new(), risk_limits, balance }
    }

    pub fn submit_order(&mut self, symbol: &str, side: OrderSide, order_type: OrderType, quantity: f64, price: Option<f64>) -> Result<String, TradingError> {
        let order_value = quantity * price.unwrap_or(0.0);
        if order_value > self.risk_limits.max_order_value {
            return Err(TradingError::RiskLimitExceeded(format!("order value {} exceeds max {}", order_value, self.risk_limits.max_order_value)));
        }
        let open_count = self.orders.iter().filter(|o| matches!(o.status, OrderStatus::Pending | OrderStatus::Submitted)).count();
        if open_count >= self.risk_limits.max_open_orders {
            return Err(TradingError::RiskLimitExceeded(format!("max open orders {} reached", self.risk_limits.max_open_orders)));
        }
        let id = uuid::Uuid::new_v4().to_string();
        self.orders.push(TradeOrder {
            id: id.clone(), symbol: symbol.to_string(), side, order_type, quantity, price, stop_price: None,
            status: OrderStatus::Pending, created_at: Utc::now(), filled_at: None, filled_price: None,
        });
        Ok(id)
    }

    pub fn cancel_order(&mut self, order_id: &str) -> Result<(), TradingError> {
        let order = self.orders.iter_mut().find(|o| o.id == order_id).ok_or_else(|| TradingError::OrderNotFound(order_id.to_string()))?;
        if matches!(order.status, OrderStatus::Pending | OrderStatus::Submitted) {
            order.status = OrderStatus::Cancelled;
            Ok(())
        } else {
            Err(TradingError::InvalidOrder(format!("cannot cancel order in {:?} state", order.status)))
        }
    }

    pub fn get_positions(&self) -> &[Position] { &self.positions }
    pub fn get_orders(&self) -> &[TradeOrder] { &self.orders }
    pub fn get_balance(&self) -> f64 { self.balance }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_market_order() {
        let mut agent = TradingAgent::new(100000.0, RiskLimits::default());
        let id = agent.submit_order("AAPL", OrderSide::Buy, OrderType::Market, 10.0, Some(150.0)).unwrap();
        assert!(!id.is_empty());
        assert_eq!(agent.get_orders().len(), 1);
    }

    #[test]
    fn test_risk_limit_order_value() {
        let mut agent = TradingAgent::new(100000.0, RiskLimits { max_order_value: 1000.0, ..Default::default() });
        assert!(agent.submit_order("AAPL", OrderSide::Buy, OrderType::Market, 100.0, Some(150.0)).is_err());
    }

    #[test]
    fn test_risk_limit_max_open_orders() {
        let mut agent = TradingAgent::new(100000.0, RiskLimits { max_open_orders: 2, ..Default::default() });
        agent.submit_order("AAPL", OrderSide::Buy, OrderType::Market, 1.0, Some(100.0)).unwrap();
        agent.submit_order("GOOG", OrderSide::Buy, OrderType::Market, 1.0, Some(100.0)).unwrap();
        assert!(agent.submit_order("MSFT", OrderSide::Buy, OrderType::Market, 1.0, Some(100.0)).is_err());
    }

    #[test]
    fn test_cancel_order() {
        let mut agent = TradingAgent::new(100000.0, RiskLimits::default());
        let id = agent.submit_order("AAPL", OrderSide::Buy, OrderType::Limit, 10.0, Some(150.0)).unwrap();
        agent.cancel_order(&id).unwrap();
        assert_eq!(agent.get_orders()[0].status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_cancel_nonexistent_order() {
        let mut agent = TradingAgent::new(100000.0, RiskLimits::default());
        assert!(agent.cancel_order("nonexistent").is_err());
    }

    #[test]
    fn test_order_side_serde() {
        let side = OrderSide::Buy;
        let json = serde_json::to_string(&side).unwrap();
        let deserialized: OrderSide = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, OrderSide::Buy);
    }

    #[test]
    fn test_market_data_serde() {
        let data = MarketData { symbol: "AAPL".to_string(), price: 150.0, bid: Some(149.9), ask: Some(150.1), volume: 1000000.0, timestamp: Utc::now() };
        let json = serde_json::to_string(&data).unwrap();
        let deserialized: MarketData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.symbol, "AAPL");
    }

    #[test]
    fn test_initial_balance() {
        let agent = TradingAgent::new(50000.0, RiskLimits::default());
        assert_eq!(agent.get_balance(), 50000.0);
    }
}
