use actix::{
    Actor, Addr, Handler, Message,
    dev::ToEnvelope,
};
use actix_web::rt::task::spawn_blocking;
use amiquip::{Channel, Connection, ConfirmSmoother, ConsumerMessage, ConsumerOptions, Exchange, QueueDeclareOptions, Publish};
use log::{error, info};
use serde::{de::DeserializeOwned, Serialize};

pub fn spawn_rabbit_consumer<T, A>(act: Addr<A>, connection: &mut Connection, queue_name: &'static str)
where
    T: DeserializeOwned + Message + Send,
    A: Handler<T>,
    <T as Message>::Result: Send,
    <A as Actor>::Context: ToEnvelope<A, T>
{
    info!("Rabbit consumer starting...");

    // Open a channel - None says let the library choose the channel ID.
    let channel = match connection.open_channel(None) {
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

pub enum SendError {
    Channel(amiquip::Error),
    Serde(serde_json::Error),
    Publish(amiquip::Error),
    Confirm(crossbeam_channel::RecvError),
}

pub fn send_and_wait_for_ack(msg: impl Serialize, channel: &Channel, queue_name: &'static str) -> Result<(), SendError> {
    // Mock responding with PASS
    let exchange = Exchange::direct(channel);

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

    // create a confirm smoother so we can process perfectly sequential confirmations
    let mut confirm_smoother = ConfirmSmoother::new();


    // Serialize struct
    let data = match serde_json::to_string(&msg) {
        Ok(data) => data,
        Err(err) => return Err(SendError::Serde(err))
    };

    // Publish message to the queue.
    match exchange.publish(Publish::new(data.as_bytes(), queue_name)) {
        Ok(_) => {
            info!("Queue {}: Message published.", queue_name);
        },
        Err(err) => {
            error!("Queue {}: Error while publishing message to rabbitmq: {}", queue_name, err);
            return Err(SendError::Publish(err))
        }
    };

    info!("Queue {}: Waiting for confirmation...", queue_name);
    // wait for confirmation from the server for those 1 messages
    let mut confirmed = 0;
    while confirmed == 0 {
        let confirm = match confirm_listener.recv() {
            Ok(confirm) => {
                info!("Confirmed!");
                confirm
            },
            Err(err) => {
                error!("Queue {}: Error while confirming recv: {:?}", queue_name, err);
                return Err(SendError::Confirm(err))
            }
        };
        println!("got raw confirm {:?} from server", confirm);
        for confirm in confirm_smoother.process(confirm) {
            info!("Queue {}: Message confirmed: {:?}", queue_name, confirm);
            confirmed += 1;
        }
    };

    Ok(())
}
