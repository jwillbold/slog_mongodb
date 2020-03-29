[![Build Status](https://travis-ci.com/jwillbold/slog_mongodb.svg?token=hPh87VpFt3MQPwdySdkS&branch=master)](https://travis-ci.com/jwillbold/slog_mongodb)
[![codecov](https://codecov.io/gh/jwillbold/slog_mongodb/branch/master/graph/badge.svg)](https://codecov.io/gh/jwillbold/slog_mongodb)

# slog_mongodb

MongoDB extension for Rust's logging library [slog-rs](https://github.com/slog-rs/slog).
Serializes [slog](https://github.com/slog-rs/slog) messages to [BSON](https://github.com/mongodb/bson-rust) documents and stores them in a [MongoDB collection](https://github.com/mongodb/mongo-rust-driver).
To reduce the stress on the database, the logged messages are buffered and only sent in configurable time intervals.

## Usage

```Rust
use slog::*;

fn main() {
    let client = mongodb::Client::with_uri_str("mongodb://localhost:27017/").unwrap();
    let db = client.database("some_db");
    let logs = db.collection("logs");

    let drain = slog_mongodb::MongoDBDrain::new(logs, std::time::Duration::from_secs(5)).fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = Logger::root(drain, o!());
    info!(log, "Hello MongoDB!");
}
```

### Notes
By default, the logged messages contain the following values:
 - "ts": RFC3339 timestamp
 - "level": "TRCE", "INFO", "WARN", "ERRO" or "CRIT"
 - "leveli": 4 - 0
 - "msg": the log message

This behavior can be changed by constructing the drain as follows:

```Rust
use slog::*;

fn main() {
    let client = mongodb::Client::with_uri_str("mongodb://localhost:27017/").unwrap();
    let db = client.database("some_db");
    let logs = db.collection("logs");

    let drain = MongoDBDrainBuilder::new(logs, std::time::Duration::from_secs(5))
            .add_key_value(o!("key" => "value")).build();
    let drain = slog_async::Async::new(drain).build().fuse();

    let log = Logger::root(drain, o!());
    info!(log, "Hello MongoDB!");
}
```

### Credits
The serde serialization as well as the overall design is copied from [slog-json](https://github.com/slog-rs/json).