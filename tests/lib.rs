extern crate mongodb;
extern crate bson;
extern crate slog;
extern crate slog_async;
extern crate slog_mongodb;

#[cfg(test)]
mod test {
    use slog::*;
    use bson::doc;
    use std::time::Duration;

    #[test]
    fn test_interval_inserts() {
        let client = mongodb::Client::with_uri_str("mongodb://localhost:27017").unwrap();
        let db = client.database("some_db");
        let logs = db.collection("logs");

        let drain = slog_mongodb::MongoDBDrain::new(logs.clone(), std::time::Duration::from_secs(10)).fuse();
        let drain = slog_async::Async::new(drain).build().fuse();

        let log = Logger::root(drain, o!());

        // 1.) Insert a few messages and check that they are not yet in the database
        trace!(log, "Trace message"; "key" => "value");
        info!(log, "Info message"; "key" => "value");
        warn!(log, "Warning message"; "key" => "value");
        error!(log, "Error message"; "key" => "value");
        crit!(log, "Critical error message"; "key" => "value");

        assert_eq!(logs.find(doc!{}, None).unwrap().count(), 0);

        std::thread::sleep(Duration::from_secs(12));

        assert_eq!(logs.find(doc!{}, None).unwrap().count(), 0);

        info!(log, "Insert trigger"; "key" => "value");

        assert_eq!(logs.find(doc!{}, None).unwrap().count(), 6);
    }
}
