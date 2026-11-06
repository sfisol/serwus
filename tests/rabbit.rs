//! `utils::rabbit` against RabbitMQ started in a container, so Docker has to be running.

#![cfg(feature = "rabbit")]

use std::{collections::HashMap, time::Duration};

use actix::{Actor, ActorContext, Context, Handler, Message};
use actix_web::rt::time::{sleep, timeout};
use futures::{StreamExt, channel::mpsc};
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties,
    options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
    types::{AMQPValue, FieldTable},
};
use serde::{Deserialize, Serialize};
use serwus::utils::rabbit::{SendError, send_and_wait_for_ack, spawn_rabbit_consumer};
use testcontainers_modules::{
    rabbitmq::RabbitMq,
    testcontainers::{
        core::{CmdWaitFor, ExecCommand},
        runners::AsyncRunner,
    },
};

const CONSUME: &str = "serwus-consume";
const DEAD: &str = "serwus-dead";

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
struct Ping(u32);

/// Hands every [`Ping`] it gets over to the test.
struct Collector(mpsc::UnboundedSender<Ping>);

impl Actor for Collector {
    type Context = Context<Self>;
}

impl Handler<Ping> for Collector {
    type Result = ();

    fn handle(&mut self, msg: Ping, _: &mut Self::Context) {
        self.0.unbounded_send(msg).expect("test stopped listening");
    }
}

/// Stops as soon as it starts, so nothing can be handed over to it.
struct Stopped;

impl Actor for Stopped {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

impl Handler<Ping> for Stopped {
    type Result = ();

    fn handle(&mut self, _: Ping, _: &mut Self::Context) {}
}

/// One container for all the cases, since RabbitMQ takes a while to start.
#[actix_web::test]
async fn rabbit() {
    let container = RabbitMq::default()
        .start()
        .await
        .expect("can't start the RabbitMQ container (is Docker running?)");
    // A policy rather than queue arguments, which the consumer would declare differently.
    container
        .exec(
            ExecCommand::new([
                "rabbitmqctl",
                "set_policy",
                "dead-letter",
                &format!("^{CONSUME}$"),
                &format!(r#"{{"dead-letter-exchange":"","dead-letter-routing-key":"{DEAD}"}}"#),
                "--apply-to",
                "queues",
            ])
            .with_cmd_ready_condition(CmdWaitFor::exit_code(0)),
        )
        .await
        .unwrap();
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5672).await.unwrap();
    let conn = Connection::connect(
        &format!("amqp://{host}:{port}"),
        ConnectionProperties::default(),
    )
    .await
    .unwrap();

    consumer_gets_published_messages(&conn).await;
    stopped_actor_gets_messages_requeued(&conn).await;
    refused_message_is_nack(&conn).await;
    unserializable_message_is_serde_error(&conn).await;
}

async fn consumer_gets_published_messages(conn: &Connection) {
    const PUBLISH: &str = "serwus-publish";

    let channel = conn.create_channel().await.unwrap();
    // The consumer declares it too, but asynchronously, and what is published before that is lost.
    declare(&channel, CONSUME, FieldTable::default()).await;
    declare(&channel, DEAD, FieldTable::default()).await;

    let (tx, mut rx) = mpsc::unbounded();
    spawn_rabbit_consumer::<Ping, _>(
        Collector(tx).start(),
        conn.create_channel().await.unwrap(),
        CONSUME,
        Some(PUBLISH),
    );

    channel
        .basic_publish(
            "".into(),
            CONSUME.into(),
            BasicPublishOptions::default(),
            b"not json",
            BasicProperties::default(),
        )
        .await
        .unwrap();
    send_and_wait_for_ack(Ping(1), &channel, CONSUME)
        .await
        .unwrap();
    send_and_wait_for_ack(Ping(2), &channel, CONSUME)
        .await
        .unwrap();

    let received: Vec<Ping> = timeout(Duration::from_secs(10), rx.by_ref().take(2).collect())
        .await
        .expect("the consumer did not hand the messages over");
    assert_eq!(received, [Ping(1), Ping(2)]);

    // The consumer declares both queues before it gets anything.
    let passive = QueueDeclareOptions {
        passive: true,
        ..Default::default()
    };
    channel
        .queue_declare(PUBLISH.into(), passive, FieldTable::default())
        .await
        .expect("the consumer did not declare the publish queue");

    let mut dead = channel
        .basic_consume(
            DEAD.into(),
            "".into(),
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .unwrap();
    let rejected = timeout(Duration::from_secs(10), dead.next())
        .await
        .expect("the consumer did not reject the message that is not JSON")
        .unwrap()
        .unwrap();
    assert_eq!(rejected.data, b"not json");
}

async fn stopped_actor_gets_messages_requeued(conn: &Connection) {
    const QUEUE: &str = "serwus-orphan";

    let channel = conn.create_channel().await.unwrap();
    declare(&channel, QUEUE, FieldTable::default()).await;

    // Kept open here, so that the message goes back to the queue only if the consumer requeues it.
    let consumer_channel = conn.create_channel().await.unwrap();
    spawn_rabbit_consumer::<Ping, _>(Stopped.start(), consumer_channel.clone(), QUEUE, None);
    wait_for_counts(&channel, QUEUE, 0, 1).await;

    send_and_wait_for_ack(Ping(4), &channel, QUEUE)
        .await
        .unwrap();
    wait_for_counts(&channel, QUEUE, 1, 0).await;
    drop(consumer_channel);
}

async fn refused_message_is_nack(conn: &Connection) {
    const FULL: &str = "serwus-full";

    let channel = conn.create_channel().await.unwrap();
    let mut args = FieldTable::default();
    args.insert("x-max-length".into(), AMQPValue::LongInt(0));
    args.insert(
        "x-overflow".into(),
        AMQPValue::LongString("reject-publish".into()),
    );
    declare(&channel, FULL, args).await;

    let result = send_and_wait_for_ack(Ping(3), &channel, FULL).await;
    assert!(matches!(result, Err(SendError::Nack)), "{result:?}");
}

async fn unserializable_message_is_serde_error(conn: &Connection) {
    let channel = conn.create_channel().await.unwrap();
    // JSON object keys have to be strings.
    let msg = HashMap::from([((1, 2), 3)]);

    let result = send_and_wait_for_ack(msg, &channel, "serwus-unused").await;
    assert!(matches!(result, Err(SendError::Serde(_))), "{result:?}");
}

/// Waits until `queue` has `messages` ready and `consumers` consumers.
async fn wait_for_counts(channel: &Channel, queue: &str, messages: u32, consumers: u32) {
    let passive = QueueDeclareOptions {
        passive: true,
        ..Default::default()
    };
    let counts = async {
        loop {
            let queue = channel
                .queue_declare(queue.into(), passive, FieldTable::default())
                .await
                .unwrap();
            if (queue.message_count(), queue.consumer_count()) == (messages, consumers) {
                return;
            }
            sleep(Duration::from_millis(100)).await;
        }
    };
    timeout(Duration::from_secs(10), counts)
        .await
        .unwrap_or_else(|_| {
            panic!("{queue} never got {messages} messages ready and {consumers} consumers")
        });
}

async fn declare(channel: &Channel, queue: &str, args: FieldTable) {
    channel
        .queue_declare(queue.into(), QueueDeclareOptions::default(), args)
        .await
        .unwrap();
}
