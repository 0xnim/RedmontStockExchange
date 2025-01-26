-- Core market entities
CREATE TABLE instruments (
                             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                             symbol VARCHAR(20) NOT NULL UNIQUE,
                             name VARCHAR(255) NOT NULL,
                             type VARCHAR(20) NOT NULL CHECK (type IN ('STOCK', 'ETF', 'BOND', 'COMMODITY')),
                             status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'SUSPENDED', 'DELISTED')),
                             lot_size INTEGER NOT NULL DEFAULT 1,
                             tick_size DECIMAL(10,4) NOT NULL,
                             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                             updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Broker accounts (your users)
CREATE TABLE brokers (
                         id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                         broker_code VARCHAR(20) NOT NULL UNIQUE,
                         name VARCHAR(255) NOT NULL,
                         status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE', 'SUSPENDED', 'TERMINATED')),
                         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Broker's cash positions (for clearing/settlement)
CREATE TABLE cash_positions (
                                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                broker_id UUID NOT NULL REFERENCES brokers(id),
                                currency VARCHAR(3) NOT NULL DEFAULT 'USD',
                                total_balance DECIMAL(20,4) NOT NULL DEFAULT 0,
                                locked_balance DECIMAL(20,4) NOT NULL DEFAULT 0,
                                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                CONSTRAINT positive_balances CHECK (total_balance >= 0 AND locked_balance >= 0),
                                CONSTRAINT locked_less_than_total CHECK (locked_balance <= total_balance)
);

-- Broker's security positions
CREATE TABLE security_positions (
                                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                    broker_id UUID NOT NULL REFERENCES brokers(id),
                                    instrument_id UUID NOT NULL REFERENCES instruments(id),
                                    total_quantity DECIMAL(20,4) NOT NULL DEFAULT 0,
                                    locked_quantity DECIMAL(20,4) NOT NULL DEFAULT 0,
                                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                    CONSTRAINT positive_quantities CHECK (total_quantity >= 0 AND locked_quantity >= 0),
                                    CONSTRAINT locked_less_than_total CHECK (locked_quantity <= total_quantity),
                                    UNIQUE(broker_id, instrument_id)
);

-- Order book
CREATE TABLE orders (
                        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                        broker_id UUID NOT NULL REFERENCES brokers(id),
                        instrument_id UUID NOT NULL REFERENCES instruments(id),
                        order_type VARCHAR(20) NOT NULL CHECK (order_type IN ('LIMIT', 'MARKET')),
                        side VARCHAR(4) NOT NULL CHECK (side IN ('BUY', 'SELL')),
                        status VARCHAR(20) NOT NULL DEFAULT 'PENDING'
                            CHECK (status IN ('PENDING', 'PARTIAL', 'FILLED', 'CANCELLED', 'REJECTED')),
                        price DECIMAL(20,4),
                        original_quantity DECIMAL(20,4) NOT NULL,
                        remaining_quantity DECIMAL(20,4) NOT NULL,
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        CONSTRAINT valid_quantities CHECK (
                            remaining_quantity >= 0 AND
                            remaining_quantity <= original_quantity AND
                            original_quantity > 0
                            ),
                        CONSTRAINT market_order_no_price CHECK (
                            (order_type = 'MARKET' AND price IS NULL) OR
                            (order_type = 'LIMIT' AND price IS NOT NULL AND price > 0)
                            )
);

-- Trade executions
CREATE TABLE trades (
                        id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                        instrument_id UUID NOT NULL REFERENCES instruments(id),
                        buyer_order_id UUID NOT NULL REFERENCES orders(id),
                        seller_order_id UUID NOT NULL REFERENCES orders(id),
                        buyer_broker_id UUID NOT NULL REFERENCES brokers(id),
                        seller_broker_id UUID NOT NULL REFERENCES brokers(id),
                        price DECIMAL(20,4) NOT NULL,
                        quantity DECIMAL(20,4) NOT NULL,
                        execution_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        status VARCHAR(20) NOT NULL DEFAULT 'PENDING_SETTLEMENT'
                            CHECK (status IN ('PENDING_SETTLEMENT', 'SETTLED', 'FAILED')),
                        settlement_time TIMESTAMPTZ,
                        CONSTRAINT positive_trade_values CHECK (price > 0 AND quantity > 0)
);

-- Indices for performance
CREATE INDEX idx_orders_instrument_status_price ON orders(instrument_id, status, price)
    WHERE status IN ('PENDING', 'PARTIAL');
CREATE INDEX idx_trades_settlement_status ON trades(status)
    WHERE status = 'PENDING_SETTLEMENT';
CREATE INDEX idx_security_positions_broker ON security_positions(broker_id);
CREATE INDEX idx_cash_positions_broker ON cash_positions(broker_id);