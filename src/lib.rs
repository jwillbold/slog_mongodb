extern crate mongodb;
extern crate bson;
extern crate serde;
extern crate slog;
extern crate chrono;

mod slog_serde;

use std::{io, time};
use std::cell::RefCell;

use slog::{FnValue, PushFnValue};
use slog::{OwnedKVList, KV, SendSyncRefUnwindSafeKV};
use slog::{Record, o};

use slog_serde::{SerdeSerializer};

use mongodb::{Collection, options::InsertManyOptions};
use std::time::Instant;


pub struct MongoDBDrain {
    values: Vec<OwnedKVList>,
    collection: Collection,
    buffer: RefCell<Vec<bson::Document>>,
    buffer_size: usize,
    max_duration: time::Duration,
    last_drained: RefCell<time::Instant>
}

impl MongoDBDrain {
    pub fn new(collection: Collection, buffer_size: usize, max_duration: time::Duration) -> MongoDBDrain {
        MongoDBDrainBuilder::new(collection, buffer_size, max_duration).with_default_keys().build()
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
            if self.buffer_size <= 1{
                let _ = self.collection.insert_one(log, None)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            } else {
                self.buffer.borrow_mut().push(log);
                if self.buffer.borrow().len() >= self.buffer_size ||
                    time::Instant::now().saturating_duration_since(*self.last_drained.borrow()) >= self.max_duration
                {
                    self.collection.insert_many(self.buffer.borrow_mut().drain(..),
                                                Some(InsertManyOptions::builder().ordered(false).build())
                    ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    self.last_drained.replace(Instant::now());
                }
            }
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Can only store BSON documents (Bson::Document)"))
        }
    }
}


pub struct MongoDBDrainBuilder {
    values: Vec<OwnedKVList>,
    collection: Collection,
    buffer_size: usize,
    max_duration: time::Duration,
}

impl MongoDBDrainBuilder {
    fn new(collection: Collection, buffer_size: usize, max_duration: time::Duration) -> Self {
        MongoDBDrainBuilder {
            values: vec![],
            collection,
            buffer_size,
            max_duration
        }
    }

    /// Build `MongoDBDrain` `Drain`
    pub fn build(self) -> MongoDBDrain {
        MongoDBDrain {
            values: self.values,
            collection: self.collection,
            buffer: RefCell::new(Vec::with_capacity(self.buffer_size)),
            buffer_size: self.buffer_size,
            max_duration: self.max_duration,
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
