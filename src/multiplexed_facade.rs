use crate::functions::{CachedScript, RsmqFunctions};
use crate::r#trait::RsmqConnection;
use crate::types::{RedisBytes, RsmqMessage, RsmqOptions, RsmqQueueAttributes};
use crate::RsmqResult;
use core::convert::TryFrom;
use core::marker::PhantomData;
use std::time::Duration;

#[derive(Clone)]
struct RedisConnection(redis::aio::MultiplexedConnection);

impl std::fmt::Debug for RedisConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MultiplexedRedisAsyncConnnection")
    }
}

#[derive(Debug, Clone)]
pub struct Rsmq {
    connection: RedisConnection,
    functions: RsmqFunctions<redis::aio::MultiplexedConnection>,
    scripts: CachedScript,
}

impl Rsmq {
    /// Creates a new RSMQ instance, including its connection
    pub async fn new(options: RsmqOptions) -> RsmqResult<Rsmq> {
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

        let connection = client.get_multiplexed_async_connection().await?;

        Rsmq::new_with_connection(connection, options.realtime, Some(&options.ns)).await
    }

    /// Special method for when you already have a redis-rs connection and you don't want redis_async to create a new one.
    pub async fn new_with_connection(
        mut connection: redis::aio::MultiplexedConnection,
        realtime: bool,
        ns: Option<&str>,
    ) -> RsmqResult<Rsmq> {
        let functions = RsmqFunctions {
            ns: ns.unwrap_or("rsmq").to_string(),
            realtime,
            conn: PhantomData,
        };

        let scripts = functions.load_scripts(&mut connection).await?;

        Ok(Rsmq {
            connection: RedisConnection(connection),
            functions,
            scripts,
        })
    }

    /// Returns the connection to the Redis server
    pub fn get_connection(&self) -> redis::aio::MultiplexedConnection {
        self.connection.0.clone()
    }
}

impl RsmqConnection for Rsmq {
    async fn change_message_visibility(
        &self,
        qname: &str,
        message_id: &str,
        hidden: Duration,
    ) -> RsmqResult<()> {
        let mut conn = self.get_connection();
        self.functions
            .change_message_visibility(&mut conn, qname, message_id, hidden, &self.scripts)
            .await
    }

    async fn create_queue(
        &self,
        qname: &str,
        hidden: Option<Duration>,
        delay: Option<Duration>,
        maxsize: Option<i32>,
    ) -> RsmqResult<()> {
        let mut conn = self.get_connection();
        self.functions
            .create_queue(&mut conn, qname, hidden, delay, maxsize)
            .await
    }

    async fn delete_message(&self, qname: &str, id: &str) -> RsmqResult<bool> {
        let mut conn = self.get_connection();
        self.functions.delete_message(&mut conn, qname, id).await
    }
    async fn delete_queue(&self, qname: &str) -> RsmqResult<()> {
        let mut conn = self.get_connection();
        self.functions.delete_queue(&mut conn, qname).await
    }
    async fn get_queue_attributes(&self, qname: &str) -> RsmqResult<RsmqQueueAttributes> {
        let mut conn = self.get_connection();
        self.functions.get_queue_attributes(&mut conn, qname).await
    }

    async fn list_queues(&self) -> RsmqResult<Vec<String>> {
        let mut conn = self.get_connection();
        self.functions.list_queues(&mut conn).await
    }

    async fn pop_message<E: TryFrom<RedisBytes, Error = Vec<u8>>>(
        &self,
        qname: &str,
    ) -> RsmqResult<Option<RsmqMessage<E>>> {
        let mut conn = self.get_connection();
        self.functions
            .pop_message::<E>(&mut conn, qname, &self.scripts)
            .await
    }

    async fn receive_message<E: TryFrom<RedisBytes, Error = Vec<u8>>>(
        &self,
        qname: &str,
        hidden: Option<Duration>,
    ) -> RsmqResult<Option<RsmqMessage<E>>> {
        let mut conn = self.get_connection();
        self.functions
            .receive_message::<E>(&mut conn, qname, hidden, &self.scripts)
            .await
    }

    async fn send_message<E: Into<RedisBytes> + Send>(
        &self,
        qname: &str,
        message: E,
        delay: Option<Duration>,
    ) -> RsmqResult<String> {
        let mut conn = self.get_connection();
        self.functions
            .send_message(&mut conn, qname, message, delay)
            .await
    }

    async fn set_queue_attributes(
        &self,
        qname: &str,
        hidden: Option<Duration>,
        delay: Option<Duration>,
        maxsize: Option<i64>,
    ) -> RsmqResult<RsmqQueueAttributes> {
        let mut conn = self.get_connection();
        self.functions
            .set_queue_attributes(&mut conn, qname, hidden, delay, maxsize)
            .await
    }
}
