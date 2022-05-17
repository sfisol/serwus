use actix::{
    Actor, Addr, Handler, Message,
    dev::ToEnvelope,
};
use actix_web::rt::task::spawn_blocking;
use amiquip::{Channel, Confirm, Connection, ConfirmSmoother, ConsumerMessage, ConsumerOptions, Exchange, QueueDeclareOptions, Publish};
use log::{error, info};
use serde::{de::DeserializeOwned, Serialize};

pub fn spawn_rabbit_consumer<T, A>(act: Addr<A>, channel: Channel, consume_queue: &'static str, publish_queue: Option<&'static str>)
where
    T: DeserializeOwned + Message + Send,
    A: Handler<T>,
    <T as Message>::Result: Send,
    <A as Actor>::Context: ToEnvelope<A, T>
{
    info!("Rabbit consumer starting...");

    spawn_blocking(move || {
        if let Some(publish_queue) = publish_queue {
            match channel.queue_declare(publish_queue, QueueDeclareOptions::default()) {
                Ok(_) => (),
                Err(err) => panic!("Error while declaring publisher queue: {}", err),
            };
        }

        let queue = match channel.queue_declare(consume_queue, QueueDeclareOptions::default()) {
            Ok(q) => q,
            Err(err) => panic!("Error while declaring consumer queue: {}", err),
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
                    println!("Body = {:?}", String::from_utf8_lossy(body));
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

#[derive(Debug)]
pub enum SendError {
    Channel(amiquip::Error),
    Serde(serde_json::Error),
    Publish(amiquip::Error),
    Confirm(crossbeam_channel::RecvError),
}

pub fn open_channel_and_send_and_wait_for_ack(msg: impl Serialize, connection: &mut Connection, routing_key: &'static str) -> Result<(), SendError> {
    let channel = connection.open_channel(None)
        .map_err(SendError::Channel)?;

    let confirm_listener = get_confirm_listener(&channel)?;

    send_and_wait_for_ack(msg, &channel, &confirm_listener, routing_key)
}

pub fn get_confirm_listener(channel: &Channel) -> Result<crossbeam_channel::Receiver<Confirm>, SendError> {
    // register a pub confirm listener before putting the channel into confirm mode
    let confirm_listener = match channel.listen_for_publisher_confirms() {
        Ok(c_l) => c_l,
        Err(err) => {
            error!("Error while registering confirm listener in rabbitmq: {}", err);
            return Err(SendError::Channel(err))
        }
    };

    // put channel in confirm mode
    channel.enable_publisher_confirms().unwrap(); // TODO:

    Ok(confirm_listener)
}

pub fn send_and_wait_for_ack(msg: impl Serialize, channel: &Channel, confirm_listener: &crossbeam_channel::Receiver<Confirm>, routing_key: &'static str) -> Result<(), SendError> {
    // Mock responding with PASS
    let exchange = Exchange::direct(channel);

    // create a confirm smoother so we can process perfectly sequential confirmations
    let mut confirm_smoother = ConfirmSmoother::new();

    // Serialize struct
    let data = match serde_json::to_string(&msg) {
        Ok(data) => data,
        Err(err) => return Err(SendError::Serde(err))
    };

    // Publish message to the queue.
    match exchange.publish(Publish::new(data.as_bytes(), routing_key)) {
        Ok(_) => {
            info!("Exchange {}: Message published.", routing_key);
        },
        Err(err) => {
            error!("Exchange {}: Error while publishing message to rabbitmq: {}", routing_key, err);
            return Err(SendError::Publish(err))
        }
    };

    info!("Exchange {}: Waiting for confirmation...", routing_key);
    // wait for confirmation from the server for those 1 messages
    let mut confirmed = 0;
    while confirmed == 0 {
        let confirm = match confirm_listener.recv() {
            Ok(confirm) => {
                info!("Confirmed!");
                confirm
            },
            Err(err) => {
                error!("Exchange {}: Error while confirming recv: {:?}", routing_key, err);
                return Err(SendError::Confirm(err))
            }
        };
        println!("got raw confirm {:?} from server", confirm);
        for confirm in confirm_smoother.process(confirm) {
            info!("Exchange {}: Message confirmed: {:?}", routing_key, confirm);
            confirmed += 1;
        }
    };

    Ok(())
}
