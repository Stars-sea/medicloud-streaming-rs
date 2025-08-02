pub mod handlers;
pub mod messages;

use anyhow::{Ok, Result};
use lapin::{Channel, Connection, Consumer, options, types};

use messages::MessageContext;

use crate::messaging::messages::Message;

pub async fn add_queue(
    connection: &Connection,
    exchange_name: &str,
    queue_name: &str,
    routing_key: &str,
) -> Result<Channel> {
    let channel = connection.create_channel().await?;

    let mut exchange_options = options::ExchangeDeclareOptions::default();
    exchange_options.durable = true;

    channel
        .exchange_declare(
            exchange_name,
            lapin::ExchangeKind::Fanout,
            exchange_options,
            types::FieldTable::default(),
        )
        .await?;

    let mut queue_options = options::QueueDeclareOptions::default();
    queue_options.durable = true;

    channel
        .queue_declare(queue_name, queue_options, types::FieldTable::default())
        .await?;

    channel
        .exchange_bind(
            queue_name,
            exchange_name,
            routing_key,
            options::ExchangeBindOptions::default(),
            types::FieldTable::default(),
        )
        .await?;

    Ok(channel)
}

pub async fn add_typed_queue<TMsg: serde::Serialize + 'static>(
    connection: &Connection,
) -> Result<Channel> {
    let queue_name = type_name::<TMsg>();
    let exchange_name = type_name::<TMsg>();

    add_queue(connection, exchange_name, queue_name, "").await
}

pub async fn send_message<TMsg: serde::Serialize + Message + 'static>(
    channel: &Channel,
    message: TMsg,
) -> Result<()> {
    let exchange_name = type_name::<TMsg>();

    let envoloped_message = MessageContext::new(message);
    let payload = serde_json::to_string(&envoloped_message)?;

    channel
        .basic_publish(
            exchange_name,
            "",
            options::BasicPublishOptions::default(),
            payload.as_bytes(),
            lapin::BasicProperties::default(),
        )
        .await?;

    Ok(())
}

pub async fn add_consumer(
    connection: &Connection,
    queue_name: &str,
    consumer_tag: &str,
) -> Result<Consumer> {
    let channel = connection.create_channel().await?;

    let mut queue_options = options::QueueDeclareOptions::default();
    queue_options.durable = true;

    channel
        .queue_declare(queue_name, queue_options, types::FieldTable::default())
        .await?;

    let consumer = channel
        .basic_consume(
            queue_name,
            consumer_tag,
            options::BasicConsumeOptions::default(),
            types::FieldTable::default(),
        )
        .await?;

    Ok(consumer)
}

pub async fn add_typed_consumer<TMsg: serde::de::DeserializeOwned + 'static>(
    connection: &Connection,
) -> Result<Consumer> {
    add_consumer(connection, type_name::<TMsg>(), type_name::<TMsg>()).await
}

fn type_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.split("::").last().unwrap_or(name)
}
