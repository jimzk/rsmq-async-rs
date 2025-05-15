use crate::sync_facade::functions::{CachedScriptSync, RsmqFunctionsSync};
use crate::types::RedisBytes;
use crate::types::{RsmqMessage, RsmqOptions, RsmqQueueAttributes};
use crate::{RsmqConnectionSync, RsmqResult};
use core::convert::TryFrom;
use std::marker::PhantomData;
use std::time::Duration;

pub struct RsmqSync {
    client: redis::Client,
    functions: RsmqFunctionsSync<redis::Connection>,
    scripts: CachedScriptSync,
}

impl Clone for RsmqSync {
    fn clone(&self) -> Self {
        RsmqSync {
            client: self.client.clone(),
            functions: RsmqFunctionsSync {
                ns: self.functions.ns.clone(),
                realtime: self.functions.realtime,
                conn: PhantomData,
            },
            scripts: self.scripts.clone(),
        }
    }
}

impl RsmqSync {
    pub fn new(options: RsmqOptions) -> RsmqResult<RsmqSync> {
        let conn_info = redis::ConnectionInfo {
            addr: redis::ConnectionAddr::Tcp(options.host, options.port),
            redis: redis::RedisConnectionInfo {
                db: options.db.into(),
                username: options.username,
                password: options.password,
                protocol: options.protocol,
            },
        };

        let client = redis::Client::open(conn_info)?;

        let mut conn = client.get_connection()?;

        let functions = RsmqFunctionsSync::<redis::Connection> {
            ns: options.ns.clone(),
            realtime: options.realtime,
            conn: PhantomData,
        };

        let scripts = functions.load_scripts(&mut conn)?;

        drop(conn);

        Ok(RsmqSync {
            client: client,
            functions,
            scripts,
        })
    }

    pub fn new_with_client(
        pool: redis::Client,
        realtime: bool,
        ns: Option<&str>,
    ) -> RsmqResult<RsmqSync> {
        let mut conn = pool.get_connection()?;

        let functions = RsmqFunctionsSync::<redis::Connection> {
            ns: ns.unwrap_or("rsmq").to_string(),
            realtime,
            conn: PhantomData,
        };

        let scripts = functions.load_scripts(&mut conn)?;

        drop(conn);

        Ok(RsmqSync {
            client: pool,
            functions: RsmqFunctionsSync {
                ns: ns.unwrap_or("rsmq").to_string(),
                realtime,
                conn: PhantomData,
            },
            scripts,
        })
    }
}

impl RsmqConnectionSync for RsmqSync {
    fn change_message_visibility(
        &self,
        qname: &str,
        message_id: &str,
        hidden: Duration,
    ) -> RsmqResult<()> {
        let mut conn = self.client.get_connection()?;
        self.functions.change_message_visibility(
            &mut conn,
            qname,
            message_id,
            hidden,
            &self.scripts,
        )
    }

    fn create_queue(
        &self,
        qname: &str,
        hidden: Option<Duration>,
        delay: Option<Duration>,
        maxsize: Option<i32>,
    ) -> RsmqResult<()> {
        let mut conn = self.client.get_connection()?;
        self.functions
            .create_queue(&mut conn, qname, hidden, delay, maxsize)
    }

    fn delete_message(&self, qname: &str, id: &str) -> RsmqResult<bool> {
        let mut conn = self.client.get_connection()?;
        self.functions.delete_message(&mut conn, qname, id)
    }
    fn delete_queue(&self, qname: &str) -> RsmqResult<()> {
        let mut conn = self.client.get_connection()?;
        self.functions.delete_queue(&mut conn, qname)
    }
    fn get_queue_attributes(&self, qname: &str) -> RsmqResult<RsmqQueueAttributes> {
        let mut conn = self.client.get_connection()?;
        self.functions.get_queue_attributes(&mut conn, qname)
    }

    fn list_queues(&self) -> RsmqResult<Vec<String>> {
        let mut conn = self.client.get_connection()?;
        self.functions.list_queues(&mut conn)
    }

    fn pop_message<E: TryFrom<RedisBytes, Error = Vec<u8>>>(
        &self,
        qname: &str,
    ) -> RsmqResult<Option<RsmqMessage<E>>> {
        let mut conn = self.client.get_connection()?;
        self.functions
            .pop_message::<E>(&mut conn, qname, &self.scripts)
    }

    fn receive_message<E: TryFrom<RedisBytes, Error = Vec<u8>>>(
        &self,
        qname: &str,
        hidden: Option<Duration>,
    ) -> RsmqResult<Option<RsmqMessage<E>>> {
        let mut conn = self.client.get_connection()?;
        self.functions
            .receive_message::<E>(&mut conn, qname, hidden, &self.scripts)
    }

    fn send_message<E: Into<RedisBytes> + Send>(
        &self,
        qname: &str,
        message: E,
        delay: Option<Duration>,
    ) -> RsmqResult<String> {
        let mut conn = self.client.get_connection()?;
        self.functions
            .send_message(&mut conn, qname, message, delay)
    }

    fn set_queue_attributes(
        &self,
        qname: &str,
        hidden: Option<Duration>,
        delay: Option<Duration>,
        maxsize: Option<i64>,
    ) -> RsmqResult<RsmqQueueAttributes> {
        let mut conn = self.client.get_connection()?;
        self.functions
            .set_queue_attributes(&mut conn, qname, hidden, delay, maxsize)
    }
}
