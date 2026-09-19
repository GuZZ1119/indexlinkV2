CREATE TABLE market_price_datasets (
    id INTEGER PRIMARY KEY,
    provider TEXT NOT NULL CHECK (length(provider) BETWEEN 1 AND 80),
    market TEXT NOT NULL CHECK (market IN ('us', 'hong_kong', 'china_shanghai', 'china_shenzhen')),
    symbol TEXT NOT NULL CHECK (length(symbol) BETWEEN 1 AND 15),
    instrument_type TEXT NOT NULL CHECK (instrument_type = 'equity_or_etf'),
    currency TEXT NOT NULL CHECK (length(currency) = 3),
    timezone TEXT NOT NULL CHECK (length(timezone) BETWEEN 1 AND 64),
    adjustment TEXT NOT NULL CHECK (adjustment IN ('raw', 'split', 'dividend', 'spin_off', 'all', 'forward', 'backward')),
    requested_start TEXT NOT NULL CHECK (requested_start GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    requested_end TEXT NOT NULL CHECK (requested_end GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]' AND requested_end >= requested_start),
    fetched_at TEXT NOT NULL CHECK (fetched_at GLOB '????-??-??T??:??:??*Z'),
    dataset_version TEXT NOT NULL CHECK (length(dataset_version) BETWEEN 1 AND 80),
    checksum TEXT NOT NULL CHECK (length(checksum) = 64 AND checksum NOT GLOB '*[^0-9a-f]*'),
    UNIQUE (provider, market, symbol, adjustment, requested_start, requested_end)
);

CREATE TABLE market_price_bars (
    dataset_id INTEGER NOT NULL REFERENCES market_price_datasets(id) ON DELETE CASCADE,
    trading_date TEXT NOT NULL CHECK (trading_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    close REAL NOT NULL CHECK (close > 0.0),
    PRIMARY KEY (dataset_id, trading_date)
);

CREATE INDEX idx_market_price_bars_dataset_date
    ON market_price_bars (dataset_id, trading_date ASC);
