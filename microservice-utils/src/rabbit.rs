use actix::{
    Actor, Addr, Handler, Message,
    dev::ToEnvelope,
};
use actix_web::rt::task::spawn_blocking;
use amiquip::{Connection, ConsumerMessage, ConsumerOptions, QueueDeclareOptions};
use log::{error, info};
use serde::de::DeserializeOwned;

pub fn spawn_rabbit_consumer<T, A>(act: Addr<A>, connection: &mut Connection, queue_name: &'static str)
where
    T: DeserializeOwned + Message + Send,
    A: Handler<T>,
    <T as Message>::Result: Send,
    <A as Actor>::Context: ToEnvelope<A, T>
{
    info!("Rabbit consumer starting...");

    // Open a channel - None says let the library choose the channel ID.
    let channel = match connection.open_channel(Some(1)) {
        Ok(ch) => ch,
        Err(err) => panic!("Error while opening channel: {}", err),
    };

    spawn_blocking(move || {
        // Declare the "gambling" queue.
        let queue = match channel.queue_declare(queue_name, QueueDeclareOptions::default()) {
            Ok(q) => q,
            Err(err) => panic!("Error while declaring queue: {}", err),
        };

        // Start a consumer.
        let consumer = match queue.consume(ConsumerOptions::default()) {
            Ok(c) => c,
            Err(err) => panic!("Error while creating consumer: {}", err),
        };

        info!("Rabbit consumer started. Waiting for messages...");

        for message in consumer.receiver().iter() {
            match message {
                ConsumerMessage::Delivery(delivery) => {
                    let body = &delivery.body[..];
                    // println!("Body = {:?}", String::from_utf8_lossy(body));
                    let msg: T = match serde_json::from_slice(body) {
                            Ok(msg) => msg,
                        Err(err) => {
                            error!("Rabbit deserialize error: {}", err);
                            continue
                        },
                    };
                    // println!("({:>3}) Received [{:?}]", i, msg);

                    act.do_send(msg);

                    match consumer.ack(delivery) {
                        Ok(d) => d,
                        Err(err) => error!("Error while delivery ack: {}", err),
                    };
                }
                other => {
                    info!("Consumer ended: {:?}", other);
                    break;
                }
            }
        }
    });
}