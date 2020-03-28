// {{{ Crate docs
//! MongoDB `Drain` for `slog-rs`
//!
//! ```
//! use slog::*;
//!
//! let client = mongodb::Client::with_uri_str("mongodb://localhost:27017/").unwrap();
//! let db = client.database("some_db");
//! let logs = db.collection("logs");
//!
//! let drain = slog_mongodb::MongoDBDrain::new(logs, std::time::Duration::from_secs(5)).fuse();
//! let drain = slog_async::Async::new(drain).build().fuse();
//!
//! let log = Logger::root(drain, o!());
//! info!(log, "Logging ready!");
//! ```
// }}}

extern crate mongodb;
extern crate bson;
extern crate serde;
extern crate slog;
extern crate chrono;

mod slog_serde;

use std::{io, time};
use std::cell::RefCell;
use std::time::Instant;

use slog::{FnValue, PushFnValue};
use slog::{OwnedKVList, KV, SendSyncRefUnwindSafeKV};
use slog::{Record, o};

use slog_serde::{SerdeSerializer};

use mongodb::{Collection, options::InsertManyOptions};


/// MongoDB `Drain` for `slog-rs`.
/// Buffers incoming log messages and stores them at a configurable time interval.
///
/// ```
/// use slog::*;
///
/// let client = mongodb::Client::with_uri_str("mongodb://localhost:27017/").unwrap();
/// let db = client.database("some_db");
/// let logs = db.collection("logs");
///
/// let drain = slog_mongodb::MongoDBDrain::new(logs, std::time::Duration::from_secs(5)).fuse();
/// let drain = slog_async::Async::new(drain).build().fuse();
///
/// let log = Logger::root(drain, o!());
/// info!(log, "Logging ready!");
// ```
pub struct MongoDBDrain {
    values: Vec<OwnedKVList>,
    collection: Collection,
    buffer: RefCell<Vec<bson::Document>>,
    drain_interval: time::Duration,
    last_drained: RefCell<time::Instant>
}

impl MongoDBDrain {
    pub fn new(collection: Collection, drain_interval: time::Duration)
        -> MongoDBDrain  {
        MongoDBDrainBuilder::new(collection, drain_interval).with_default_keys().build()
    }
}

impl slog::Drain for MongoDBDrain  {
    type Ok = ();
    type Err = io::Error;

    fn log(&self, rinfo: &Record, logger_values: &OwnedKVList) -> io::Result<()> {
        let encoder = bson::Encoder::new();
        let mut serializer = SerdeSerializer::start(encoder, None)?;

        for kv in &self.values {
            kv.serialize(rinfo, &mut serializer)?;
        }

        logger_values.serialize(rinfo, &mut serializer)?;
        rinfo.kv().serialize(rinfo, &mut serializer)?;

        let res = serializer.end().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if let bson::Bson::Document(log) = res {
            self.buffer.borrow_mut().push(log);
            if time::Instant::now().saturating_duration_since(*self.last_drained.borrow()) >= self.drain_interval
            {
                self.collection.insert_many(self.buffer.borrow_mut().drain(..),
                    Some(InsertManyOptions::builder().ordered(false).build())
                ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                self.last_drained.replace(Instant::now());
            }
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Can only store BSON documents (Bson::Document)"))
        }
    }
}

/// MongoDB `Drain` builder
pub struct MongoDBDrainBuilder {
    values: Vec<OwnedKVList>,
    collection: Collection,
    drain_interval: time::Duration,
}

impl MongoDBDrainBuilder {
    fn new(collection: Collection, drain_interval: time::Duration) -> Self {
        MongoDBDrainBuilder {
            values: vec![],
            collection,
            drain_interval
        }
    }

    /// Build `MongoDBDrain` `Drain`
    pub fn build(self) -> MongoDBDrain {
        MongoDBDrain {
            values: self.values,
            collection: self.collection,
            buffer: RefCell::new(Vec::with_capacity(50)),
            drain_interval: self.drain_interval,
            last_drained: RefCell::new(Instant::now())
        }
    }

    /// Add custom values to be printed with this formatter
    pub fn add_key_value<T>(mut self, value: slog::OwnedKV<T>) -> Self
        where T: SendSyncRefUnwindSafeKV + 'static
    {
        self.values.push(value.into());
        self
    }

    /// Add default key-values:
    ///
    /// * `ts` - timestamp
    /// * `level` - record logging level name
    /// * `leveli` - record logging level integer, "Critical is the smallest and Trace the biggest value" - slog::Level, docs.rs/slog
    /// * `msg` - msg - formatted logging message
    pub fn with_default_keys(self) -> Self {
        self.add_key_value(o!(
            "ts" => PushFnValue(move |_, ser| ser.emit(chrono::Local::now().to_rfc3339())),
            "level" => FnValue(move |rinfo| rinfo.level().as_short_str()),
            "leveli" => FnValue(move |rinfo| rinfo.level().as_usize()),
            "msg" => PushFnValue(move |record, ser| ser.emit(record.msg())),
        ))
    }
}
