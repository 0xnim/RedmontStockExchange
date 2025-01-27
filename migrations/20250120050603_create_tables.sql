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
                                currency VARCHAR(3) NOT NULL DEFAULT 'RMD',
                                total_balance DECIMAL(20,4) NOT NULL DEFAULT 0,
                                locked_balance DECIMAL(20,4) NOT NULL DEFAULT 0,
                                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                CONSTRAINT positive_balances CHECK (total_balance >= 0 AND locked_balance >= 0),
                                CONSTRAINT locked_less_than_total CHECK (locked_balance <= total_balance),
                                UNIQUE(broker_id, currency)
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
                        time_in_force VARCHAR(4) NOT NULL DEFAULT 'GTC'  -- New column
                            CHECK (time_in_force IN ('GTC', 'IOC', 'FOK')),
                        status VARCHAR(20) NOT NULL DEFAULT 'PENDING'
                            CHECK (status IN ('PENDING', 'PARTIAL', 'FILLED', 'CANCELLED', 'REJECTED')),
                        price DECIMAL(20,4),
                        original_quantity DECIMAL(20,4) NOT NULL,
                        remaining_quantity DECIMAL(20,4) NOT NULL DEFAULT original_quantity,  -- Changed default *GOOD*
    -- Changes: Additional order metadata
                        client_order_id VARCHAR(50),  -- Client-provided ID
                        parent_order_id UUID REFERENCES orders(id),  -- OCO orders
                        reason TEXT,  -- Rejection/cancel reason
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
    -- Changes: Fee structure
                        buyer_fee DECIMAL(20,4),
                        seller_fee DECIMAL(20,4),
                        exchange_fee DECIMAL(20,4),
                        clearing_fee DECIMAL(20,4),
                        execution_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        status VARCHAR(20) NOT NULL DEFAULT 'PENDING_SETTLEMENT'
                            CHECK (status IN ('PENDING_SETTLEMENT', 'SETTLED', 'FAILED')),
                        settlement_time TIMESTAMPTZ,
                        CONSTRAINT positive_trade_values CHECK (price > 0 AND quantity > 0)
);

-- New: Corporate actions tracking
CREATE TABLE corporate_actions (
                                   id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                                   instrument_id UUID NOT NULL REFERENCES instruments(id),
                                   action_type VARCHAR(20) NOT NULL CHECK (action_type IN ('DIVIDEND', 'SPLIT', 'MERGER')),
                                   ex_date DATE NOT NULL,
                                   record_date DATE,
                                   details JSONB,
                                   created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- New: Settlement process tracking
CREATE TABLE settlements (
                             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                             trade_id UUID NOT NULL REFERENCES trades(id),
                             status VARCHAR(20) NOT NULL CHECK (status IN ('PENDING', 'COMPLETED', 'FAILED')),
                             settlement_type VARCHAR(20) NOT NULL CHECK (settlement_type IN ('T+1', 'T+2', 'CASH')),
                             net_amount DECIMAL(20,4),
                             failure_reason TEXT,
                             created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                             updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- New: Risk management tables
CREATE TABLE position_limits (
                                 broker_id UUID NOT NULL REFERENCES brokers(id),
                                 instrument_id UUID REFERENCES instruments(id),
                                 max_position DECIMAL(20,4) NOT NULL,
                                 max_order_value DECIMAL(20,4),
                                 currency VARCHAR(3) DEFAULT 'RMD',
                                 PRIMARY KEY (broker_id, instrument_id)
);

CREATE TABLE margin_requirements (
                                     instrument_type VARCHAR(20) PRIMARY KEY
                                         CHECK (instrument_type IN ('STOCK', 'ETF', 'BOND', 'COMMODITY')),
                                     initial_margin DECIMAL(5,2) NOT NULL,
                                     maintenance_margin DECIMAL(5,2) NOT NULL
);

-- New: Audit system
CREATE TABLE order_audit (
                             id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                             order_id UUID NOT NULL REFERENCES orders(id),
                             old_status VARCHAR(20),
                             new_status VARCHAR(20),
                             changed_by VARCHAR(50),
                             change_reason TEXT,
                             changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Changes: Automatic updated_at triggers
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_instruments_updated_at BEFORE UPDATE ON instruments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER update_brokers_updated_at BEFORE UPDATE ON brokers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER update_cash_positions_updated_at BEFORE UPDATE ON cash_positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER update_security_positions_updated_at BEFORE UPDATE ON security_positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER update_orders_updated_at BEFORE UPDATE ON orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER update_trades_updated_at BEFORE UPDATE ON trades
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Changes: Additional performance indexes
CREATE INDEX idx_orders_broker ON orders(broker_id);
CREATE INDEX idx_trades_buyer ON trades(buyer_broker_id);
CREATE INDEX idx_trades_seller ON trades(seller_broker_id);
