A stock exchange system written in Rust. For DemocracyCraft.

## Features

- Instruments (stocks, ETFs, bonds, etc.)
- Brokers (exchanges, clearing houses, etc.)
- Order book
- Trade executions
- Corporate actions tracking
- Settlement process tracking

## Database

The database is managed by [SQLx](https://github.com/launchbadge/sqlx). The SQL schema is defined in the `migrations` directory.

## Running the application

To run the application, you need to have PostgreSQL installed on your machine.