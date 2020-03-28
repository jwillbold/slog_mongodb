# slog_mongodb

Serializes the [slog](https://github.com/slog-rs/slog) key-value pairs to [BSON](https://github.com/mongodb/bson-rust) and stores it in a [MongoDB collection](https://github.com/mongodb/mongo-rust-driver).
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

### Credits
The serde serialization as well as the overall design is copied from [slog-json](https://github.com/slog-rs/json).