use super::models::*;
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug)]
pub struct OrderBook {
    instrument_id: Uuid,
    bids: BTreeMap<Decimal, Vec<Order>>,
    asks: BTreeMap<Decimal, Vec<Order>>,
    orders: HashMap<Uuid, Order>,
}

impl OrderBook {
    pub fn new(instrument_id: Uuid) -> Self {
        Self {
            instrument_id,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
        }
    }

    pub fn add_order(&mut self, mut order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        order.status = OrderStatus::PENDING;

        match order.order_type {
            OrderType::LIMIT => self.process_limit_order(order, &mut trades),
            OrderType::MARKET => self.process_market_order(order, &mut trades),
        }

        trades
    }

    fn process_limit_order(&mut self, mut order: Order, trades: &mut Vec<Trade>) {
        let price = order.price.expect("Limit orders must have a price");
        let side = order.side.clone();

        loop {
            let matching_order_opt = match side {
                OrderSide::BUY => self.get_best_ask(),
                OrderSide::SELL => self.get_best_bid(),
            };

            match matching_order_opt {
                Some((best_price, matched_order)) if self.prices_match(side.clone(), price, best_price) => {
                    let trade_quantity = order.remaining_quantity.min(matched_order.remaining_quantity);

                    trades.push(self.create_trade(
                        &order,
                        &matched_order,
                        best_price,
                        trade_quantity
                    ));

                    order.remaining_quantity -= trade_quantity;
                    order.status = if order.remaining_quantity == Decimal::ZERO {
                        OrderStatus::FILLED
                    } else {
                        OrderStatus::PARTIAL
                    };

                    self.update_matched_order(&matched_order, trade_quantity, best_price, side.clone());

                    if order.remaining_quantity == Decimal::ZERO {
                        break;
                    }
                }
                _ => break,
            }
        }

        if order.remaining_quantity > Decimal::ZERO {
            match side {
                OrderSide::BUY => self.bids.entry(price)
                    .or_insert_with(Vec::new)
                    .push(order.clone()),
                OrderSide::SELL => self.asks.entry(price)
                    .or_insert_with(Vec::new)
                    .push(order.clone()),
            }
        }

        self.orders.insert(order.id, order);
    }

    fn process_market_order(&mut self, mut order: Order, trades: &mut Vec<Trade>) {
        let side = order.side.clone();

        loop {
            let matching_order_opt = match side {
                OrderSide::BUY => self.get_best_ask(),
                OrderSide::SELL => self.get_best_bid(),
            };

            match matching_order_opt {
                Some((price, matched_order)) => {
                    let trade_quantity = order.remaining_quantity.min(matched_order.remaining_quantity);

                    trades.push(self.create_trade(
                        &order,
                        &matched_order,
                        price,
                        trade_quantity
                    ));

                    order.remaining_quantity -= trade_quantity;
                    order.status = if order.remaining_quantity == Decimal::ZERO {
                        OrderStatus::FILLED
                    } else {
                        OrderStatus::PARTIAL
                    };

                    self.update_matched_order(&matched_order, trade_quantity, price, side.clone());

                    if order.remaining_quantity == Decimal::ZERO {
                        break;
                    }
                }
                None => {
                    order.status = OrderStatus::REJECTED;
                    break;
                }
            }
        }

        if order.remaining_quantity > Decimal::ZERO {
            order.status = OrderStatus::REJECTED;
        }

        self.orders.insert(order.id, order);
    }

    pub fn cancel_order(&mut self, order_id: Uuid) -> Option<Order> {
        if let Some(order) = self.orders.get(&order_id) {
            if order.status != OrderStatus::PENDING && order.status != OrderStatus::PARTIAL {
                return None;
            }

            let price = order.price.expect("Order should have a price");
            let side = order.side.clone();

            let book = match side {
                OrderSide::BUY => &mut self.bids,
                OrderSide::SELL => &mut self.asks,
            };

            if let Some(orders) = book.get_mut(&price) {
                if let Some(pos) = orders.iter().position(|o| o.id == order_id) {
                    let cancelled_order = orders.remove(pos);
                    if orders.is_empty() {
                        book.remove(&price);
                    }

                    let mut updated_order = cancelled_order.clone();
                    updated_order.status = OrderStatus::CANCELLED;
                    self.orders.insert(order_id, updated_order.clone());

                    return Some(updated_order);
                }
            }
        }
        None
    }

    fn get_best_ask(&mut self) -> Option<(Decimal, Order)> {
        if let Some((&price, orders)) = self.asks.iter_mut().next() {
            if !orders.is_empty() {
                let order = orders[0].clone();
                return Some((price, order));
            }
        }
        None
    }

    fn get_best_bid(&mut self) -> Option<(Decimal, Order)> {
        if let Some((&price, orders)) = self.bids.iter_mut().next() {
            if !orders.is_empty() {
                let order = orders[0].clone();
                return Some((price, order));
            }
        }
        None
    }

    fn update_matched_order(&mut self, matched_order: &Order, trade_quantity: Decimal, price: Decimal, side: OrderSide) {
        let book = match side {
            OrderSide::BUY => &mut self.asks,
            OrderSide::SELL => &mut self.bids,
        };

        if let Some(orders) = book.get_mut(&price) {
            if !orders.is_empty() {
                if orders[0].remaining_quantity == trade_quantity {
                    orders.remove(0);
                    if orders.is_empty() {
                        book.remove(&price);
                    }
                } else {
                    orders[0].remaining_quantity -= trade_quantity;
                    orders[0].status = OrderStatus::PARTIAL;
                }
            }
        }

        let mut updated_order = matched_order.clone();
        updated_order.remaining_quantity -= trade_quantity;
        updated_order.status = if updated_order.remaining_quantity == Decimal::ZERO {
            OrderStatus::FILLED
        } else {
            OrderStatus::PARTIAL
        };
        self.orders.insert(updated_order.id, updated_order);
    }

    fn create_trade(&self, order: &Order, matched_order: &Order, price: Decimal, quantity: Decimal) -> Trade {
        Trade {
            id: Uuid::new_v4(),
            instrument_id: self.instrument_id,
            buyer_order_id: if order.side == OrderSide::BUY {
                order.id
            } else {
                matched_order.id
            },
            seller_order_id: if order.side == OrderSide::SELL {
                order.id
            } else {
                matched_order.id
            },
            buyer_broker_id: if order.side == OrderSide::BUY {
                order.broker_id
            } else {
                matched_order.broker_id
            },
            seller_broker_id: if order.side == OrderSide::SELL {
                order.broker_id
            } else {
                matched_order.broker_id
            },
            price,
            quantity,
            execution_time: Utc::now(),
            status: TradeStatus::PENDING_SETTLEMENT,
            settlement_time: None,
        }
    }

    fn prices_match(&self, side: OrderSide, order_price: Decimal, book_price: Decimal) -> bool {
        match side {
            OrderSide::BUY => order_price >= book_price,
            OrderSide::SELL => order_price <= book_price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    // Helper function to print a visual separator
    fn print_separator(test_name: &str) {
        println!("\n{}", "=".repeat(50));
        println!("🧪 TEST: {}", test_name);
        println!("{}\n", "=".repeat(50));
    }

    // Helper function to visualize an order
    fn visualize_order(prefix: &str, order: &Order) {
        println!("📝 {} Order:", prefix);
        println!("   ├─ ID: {}", order.id);
        println!("   ├─ Type: {:?}", order.order_type);
        println!("   ├─ Side: {:?}", order.side);
        println!("   ├─ Price: {:?}", order.price);
        println!("   ├─ Quantity: {}", order.original_quantity);
        println!("   └─ Status: {:?}", order.status);
    }

    // Helper function to visualize a trade
    fn visualize_trade(trade: &Trade) {
        println!("\n🤝 Trade Executed:");
        println!("   ├─ Price: {}", trade.price);
        println!("   ├─ Quantity: {}", trade.quantity);
        println!("   ├─ Buyer Order: {}", trade.buyer_order_id);
        println!("   └─ Seller Order: {}", trade.seller_order_id);
    }

    // Helper function to visualize the order book state
    fn visualize_order_book_state(order_book: &OrderBook) {
        println!("\n📚 Order Book State:");
        println!("   ├─ Bids: {:?}", order_book.bids);
        println!("   ├─ Asks: {:?}", order_book.asks);
        println!("   └─ Orders: {:?}", order_book.orders);
    }

    fn create_test_order(
        id: &str,
        broker_id: &str,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Decimal>,
        quantity: Decimal,
    ) -> Order {
        Order {
            id: Uuid::from_str(id).unwrap(),
            broker_id: Uuid::from_str(broker_id).unwrap(),
            instrument_id: Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap(),
            order_type,
            side,
            status: OrderStatus::PENDING,
            price,
            original_quantity: quantity,
            remaining_quantity: quantity,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_limit_order_full_match() {
        print_separator("Limit Order Full Match");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        // Create a sell limit order
        let sell_order = create_test_order(
            "00000000-0000-0000-0000-000000000002",
            "00000000-0000-0000-0000-000000000003",
            OrderSide::SELL,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(10.0),
        );

        println!("➡️ Adding Sell Order to Book:");
        visualize_order("SELL", &sell_order);

        let trades = order_book.add_order(sell_order);
        println!("\n📚 Order Book State: No trades, order added to book");

        // Create a matching buy order
        let buy_order = create_test_order(
            "00000000-0000-0000-0000-000000000004",
            "00000000-0000-0000-0000-000000000005",
            OrderSide::BUY,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(10.0),
        );

        println!("\n➡️ Adding Buy Order:");
        visualize_order("BUY", &buy_order);

        let trades = order_book.add_order(buy_order);

        println!("\n💫 Result:");
        for trade in &trades {
            visualize_trade(trade);
        }
        println!("📚 Order Book State: Empty (all orders matched)");

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(10.0));
        assert_eq!(trades[0].price, dec!(100.0));
    }

    #[test]
    fn test_limit_order_partial_match() {
        print_separator("Limit Order Partial Match");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        let sell_order = create_test_order(
            "00000000-0000-0000-0000-000000000002",
            "00000000-0000-0000-0000-000000000003",
            OrderSide::SELL,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(10.0),
        );

        println!("➡️ Adding Sell Order to Book (Quantity: 10):");
        visualize_order("SELL", &sell_order);

        order_book.add_order(sell_order);

        let buy_order = create_test_order(
            "00000000-0000-0000-0000-000000000004",
            "00000000-0000-0000-0000-000000000005",
            OrderSide::BUY,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(5.0),
        );

        println!("\n➡️ Adding Buy Order (Quantity: 5):");
        visualize_order("BUY", &buy_order);

        let trades = order_book.add_order(buy_order);

        println!("\n💫 Result:");
        for trade in &trades {
            visualize_trade(trade);
        }

        if let Some(order) = order_book.orders.get(&Uuid::from_str("00000000-0000-0000-0000-000000000002").unwrap()) {
            println!("\n📚 Order Book State:");
            println!("   Remaining Sell Order:");
            println!("   ├─ Quantity: {}", order.remaining_quantity);
            println!("   └─ Status: {:?}", order.status);
        };
    }

    #[test]
    fn test_market_order_full_execution() {
        print_separator("Market Order Full Execution");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        let sell_order = create_test_order(
            "00000000-0000-0000-0000-000000000002",
            "00000000-0000-0000-0000-000000000003",
            OrderSide::SELL,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(10.0),
        );

        println!("➡️ Adding Limit Sell Order to Book:");
        visualize_order("SELL", &sell_order);

        order_book.add_order(sell_order);

        let buy_order = create_test_order(
            "00000000-0000-0000-0000-000000000004",
            "00000000-0000-0000-0000-000000000005",
            OrderSide::BUY,
            OrderType::MARKET,
            None,
            dec!(10.0),
        );

        println!("\n➡️ Adding Market Buy Order:");
        visualize_order("BUY", &buy_order);

        let trades = order_book.add_order(buy_order);

        println!("\n💫 Result:");
        for trade in &trades {
            visualize_trade(trade);
        }
        println!("\n📚 Order Book State: Empty (all orders matched)");
    }

    #[test]
    fn test_multiple_price_levels() {
        print_separator("Multiple Price Levels");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        let sell_order_1 = create_test_order(
            "00000000-0000-0000-0000-000000000002",
            "00000000-0000-0000-0000-000000000003",
            OrderSide::SELL,
            OrderType::LIMIT,
            Some(dec!(100.0)),
            dec!(5.0),
        );

        println!("➡️ Adding First Sell Order (Price: 100):");
        visualize_order("SELL", &sell_order_1);

        let sell_order_2 = create_test_order(
            "00000000-0000-0000-0000-000000000006",
            "00000000-0000-0000-0000-000000000007",
            OrderSide::SELL,
            OrderType::LIMIT,
            Some(dec!(101.0)),
            dec!(5.0),
        );

        println!("\n➡️ Adding Second Sell Order (Price: 101):");
        visualize_order("SELL", &sell_order_2);

        order_book.add_order(sell_order_1);
        order_book.add_order(sell_order_2);

        println!("\n📚 Order Book State: Two sell orders at different prices");

        let buy_order = create_test_order(
            "00000000-0000-0000-0000-000000000004",
            "00000000-0000-0000-0000-000000000005",
            OrderSide::BUY,
            OrderType::LIMIT,
            Some(dec!(101.0)),
            dec!(10.0),
        );

        println!("\n➡️ Adding Buy Order (Quantity: 10, Price: 101):");
        visualize_order("BUY", &buy_order);

        let trades = order_book.add_order(buy_order);

        println!("\n💫 Results:");
        for (i, trade) in trades.iter().enumerate() {
            println!("\n🤝 Trade {} Executed:", i + 1);
            visualize_trade(trade);
        }
        println!("\n📚 Order Book State: Empty (all orders matched)");
    }

    #[test]
    fn test_cancel_pending_order() {
        print_separator("Cancel Pending Order");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        // Create a sell limit order
        let sell_order = Order {
            id: Uuid::new_v4(),
            broker_id: Uuid::new_v4(),
            instrument_id,
            order_type: OrderType::LIMIT,
            side: OrderSide::SELL,
            status: OrderStatus::PENDING,
            price: Some(dec!(100.0)),
            original_quantity: dec!(10.0),
            remaining_quantity: dec!(10.0),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let order_id = sell_order.id;
        visualize_order("SELL", &sell_order);

        order_book.add_order(sell_order);
        visualize_order_book_state(&order_book);

        // Cancel the order
        let cancelled_order = order_book.cancel_order(order_id).unwrap();
        visualize_order("CANCELLED", &cancelled_order);

        visualize_order_book_state(&order_book);

        assert_eq!(cancelled_order.status, OrderStatus::CANCELLED);
        assert_eq!(cancelled_order.remaining_quantity, dec!(10.0));
        assert!(order_book.asks.is_empty());
    }

    #[test]
    fn test_cancel_partially_filled_order() {
        print_separator("Cancel Partially Filled Order");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        // Create a sell limit order
        let sell_order = Order {
            id: Uuid::new_v4(),
            broker_id: Uuid::new_v4(),
            instrument_id,
            order_type: OrderType::LIMIT,
            side: OrderSide::SELL,
            status: OrderStatus::PENDING,
            price: Some(dec!(100.0)),
            original_quantity: dec!(10.0),
            remaining_quantity: dec!(10.0),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let sell_order_id = sell_order.id;
        visualize_order("SELL", &sell_order);

        order_book.add_order(sell_order);

        // Create a partial matching buy order
        let buy_order = Order {
            id: Uuid::new_v4(),
            broker_id: Uuid::new_v4(),
            instrument_id,
            order_type: OrderType::LIMIT,
            side: OrderSide::BUY,
            status: OrderStatus::PENDING,
            price: Some(dec!(100.0)),
            original_quantity: dec!(6.0),
            remaining_quantity: dec!(6.0),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        visualize_order("BUY", &buy_order);

        // This should partially fill the sell order
        order_book.add_order(buy_order);
        visualize_order_book_state(&order_book);

        // Cancel the partially filled sell order
        let cancelled_order = order_book.cancel_order(sell_order_id).unwrap();
        visualize_order("CANCELLED", &cancelled_order);

        visualize_order_book_state(&order_book);

        assert_eq!(cancelled_order.status, OrderStatus::CANCELLED);
        assert_eq!(cancelled_order.remaining_quantity, dec!(4.0));
        assert!(order_book.asks.is_empty());
    }

    #[test]
    fn test_cancel_filled_order() {
        print_separator("Cancel Filled Order");

        let instrument_id = Uuid::from_str("00000000-0000-0000-0000-000000000001").unwrap();
        let mut order_book = OrderBook::new(instrument_id);

        // Create a sell limit order
        let sell_order = Order {
            id: Uuid::new_v4(),
            broker_id: Uuid::new_v4(),
            instrument_id,
            order_type: OrderType::LIMIT,
            side: OrderSide::SELL,
            status: OrderStatus::FILLED,
            price: Some(dec!(100.0)),
            original_quantity: dec!(10.0),
            remaining_quantity: dec!(0.0),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let order_id = sell_order.id;
        visualize_order("SELL", &sell_order);

        order_book.add_order(sell_order);
        visualize_order_book_state(&order_book);

        // Attempt to cancel the filled order
        let cancelled_order = order_book.cancel_order(order_id);

        if cancelled_order.is_none() {
            println!("\n➡️ Attempt to Cancel Filled Order:");
            println!("   └─ No order was cancelled (expected behavior).");
        }

        visualize_order_book_state(&order_book);

        assert!(cancelled_order.is_none());
    }
}