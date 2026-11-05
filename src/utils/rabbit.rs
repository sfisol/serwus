//! RabbitMQ through [`lapin`]: a consumer handing messages over to an actor, and publishing that
//! waits for the broker to confirm.

use std::{error::Error, fmt};

use actix::{Actor, Addr, Handler, Message, dev::ToEnvelope};
use actix_web::rt::spawn;
use futures_util::StreamExt;
use lapin::{
    BasicProperties, Channel, Confirmation,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, ConfirmSelectOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
};
use log::{error, info};
use serde::{Serialize, de::DeserializeOwned};

/// Declares `consume_queue` (and `publish_queue`, when given) and hands every message from
/// `consume_queue`, deserialized from JSON, over to `act`. A message is acknowledged once it is
/// handed over; one that does not deserialize is logged and left unacknowledged.
///
/// The consumer runs in a task on the current actix runtime until the channel closes. Errors
/// end it and go to the log.
pub fn spawn_rabbit_consumer<T, A>(
    act: Addr<A>,
    channel: Channel,
    consume_queue: &'static str,
    publish_queue: Option<&'static str>,
) where
    T: DeserializeOwned + Message + Send + 'static,
    A: Handler<T>,
    <T as Message>::Result: Send,
    <A as Actor>::Context: ToEnvelope<A, T>,
{
    info!("Rabbit consumer starting...");

    spawn(async move {
        if let Err(err) = consume(act, channel, consume_queue, publish_queue).await {
            error!("Rabbit consumer on {consume_queue}: {err}");
        }
    });
}

async fn consume<T, A>(
    act: Addr<A>,
    channel: Channel,
    consume_queue: &'static str,
    publish_queue: Option<&'static str>,
) -> lapin::Result<()>
where
    T: DeserializeOwned + Message + Send + 'static,
    A: Handler<T>,
    <T as Message>::Result: Send,
    <A as Actor>::Context: ToEnvelope<A, T>,
{
    if let Some(publish_queue) = publish_queue {
        channel
            .queue_declare(
                publish_queue.into(),
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }
    channel
        .queue_declare(
            consume_queue.into(),
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            consume_queue.into(),
            "".into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("Rabbit consumer started. Waiting for messages...");

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let msg: T = match serde_json::from_slice(&delivery.data) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Rabbit deserialize error: {err}");
                continue;
            }
        };

        act.do_send(msg);

        if let Err(err) = delivery.acker.ack(BasicAckOptions::default()).await {
            error!("Error while delivery ack: {err}");
        }
    }

    info!("Rabbit consumer on {consume_queue} ended");
    Ok(())
}

/// Why [`send_and_wait_for_ack`] did not deliver the message.
#[derive(Debug)]
pub enum SendError {
    /// The channel could not be put into confirm mode.
    Channel(lapin::Error),
    Serde(serde_json::Error),
    Publish(lapin::Error),
    /// Waiting for the confirmation failed.
    Confirm(lapin::Error),
    /// The broker refused the message.
    Nack,
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Channel(err) => write!(f, "enabling publisher confirms: {err}"),
            Self::Serde(err) => write!(f, "serializing message: {err}"),
            Self::Publish(err) => write!(f, "publishing message: {err}"),
            Self::Confirm(err) => write!(f, "waiting for confirmation: {err}"),
            Self::Nack => f.write_str("message refused by the broker"),
        }
    }
}

impl Error for SendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Channel(err) | Self::Publish(err) | Self::Confirm(err) => Some(err),
            Self::Serde(err) => Some(err),
            Self::Nack => None,
        }
    }
}

/// Publishes `msg` as JSON to the default exchange with `routing_key` and waits until the broker
/// confirms it.
pub async fn send_and_wait_for_ack(
    msg: impl Serialize,
    channel: &Channel,
    routing_key: &str,
) -> Result<(), SendError> {
    channel
        .confirm_select(ConfirmSelectOptions::default())
        .await
        .map_err(|err| {
            error!("Error while enabling publisher confirms in rabbitmq: {err}");
            SendError::Channel(err)
        })?;

    let data = serde_json::to_vec(&msg).map_err(SendError::Serde)?;

    let confirm = channel
        .basic_publish(
            "".into(),
            routing_key.into(),
            BasicPublishOptions::default(),
            &data,
            BasicProperties::default(),
        )
        .await
        .map_err(|err| {
            error!("Exchange {routing_key}: Error while publishing message to rabbitmq: {err}");
            SendError::Publish(err)
        })?;

    info!("Exchange {routing_key}: Message published, waiting for confirmation...");

    match confirm.await {
        Ok(Confirmation::Ack(_) | Confirmation::NotRequested) => {
            info!("Exchange {routing_key}: Message confirmed");
            Ok(())
        }
        Ok(Confirmation::Nack(_)) => {
            error!("Exchange {routing_key}: Message refused by the broker");
            Err(SendError::Nack)
        }
        Err(err) => {
            error!("Exchange {routing_key}: Error while waiting for confirmation: {err}");
            Err(SendError::Confirm(err))
        }
    }
}
