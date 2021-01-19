use std::collections::HashMap;
use rand::Rng;
use std::sync::Arc;
use async_std::sync::Mutex;

use crate::error::Result;
use crate::net::messages::{Message, PacketType};
use crate::net::utility::clone_net_error;

pub type MessageSubscriberPtr = Arc<MessageSubscriber>;

pub type MessageResult = Result<Arc<Message>>;
pub type MessageSubscriptionID = u64;

macro_rules! receive_message {
    ($sub:expr, $message_type:path) => {
        {
            let wrapped_message = OwningRef::new($sub.receive().await?);

            wrapped_message.map(|msg|
                match msg {
                    $message_type(msg_detail) => {
                        msg_detail
                    },
                    _ => {
                        panic!("Filter for receive sub invalid!");
                    }
            })
        }
    };
}

trait CloneMessageResult {
    fn clone(&self) -> Self;
}

impl CloneMessageResult for Result<Arc<Message>> {
    fn clone(&self) -> Self {
        match self {
            Ok(message) => Ok(message.clone()),
            Err(err) => Err(clone_net_error(err))
        }
    }
}

pub struct MessageSubscription {
    id: MessageSubscriptionID,
    filter: PacketType,
    recv_queue: async_channel::Receiver<MessageResult>,
    parent: Arc<MessageSubscriber>
}

impl MessageSubscription {
    fn is_relevant_message(&self, message_result: &MessageResult) -> bool {
        match message_result {
            Ok(message) => {
                let packet_type = message.packet_type();

                // Apply the filter
                packet_type == self.filter
            }
            Err(_) => {
                // Propagate all errors
                true
            }
        }
    }

    pub async fn receive(&self) -> MessageResult {
        loop {
            let message_result = self.recv_queue.recv().await;

            match message_result {
                Ok(message_result) => {
                    if self.clone().is_relevant_message(&message_result) {
                        return message_result;
                    }
                }
                Err(err) => {
                    panic!("MessageSubscription::receive() recv_queue failed! {}", err);
                }
            }
        }
    }

    // Must be called manually since async Drop is not possible in Rust
    pub async fn unsubscribe(&self) {
        self.parent.clone().unsubscribe(self.id).await
    }
}

pub struct MessageSubscriber {
    subs: Mutex<HashMap<u64, async_channel::Sender<MessageResult>>>,
}

impl MessageSubscriber {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            subs: Mutex::new(HashMap::new()),
        })
    }

    pub fn random_id() -> MessageSubscriptionID {
        let mut rng = rand::thread_rng();
        rng.gen()
    }

    pub async fn subscribe(self: Arc<Self>, packet_type: PacketType) -> MessageSubscription {
        let (sender, recvr) = async_channel::unbounded();

        let sub_id = Self::random_id();

        self.subs.lock().await.insert(sub_id, sender);

        MessageSubscription {
            id: sub_id,
            filter: packet_type,
            recv_queue: recvr,
            parent: self.clone()
        }
    }

    async fn unsubscribe(self: Arc<Self>, sub_id: MessageSubscriptionID) {
        self.subs.lock().await.remove(&sub_id);
    }

    pub async fn notify(&self, message_result: Result<Arc<Message>>) {
        for sub in (*self.subs.lock().await).values() {
            match sub.send(message_result.clone()).await {
                Ok(()) => {},
                Err(err) => {
                    panic!("Error returned sending message in notify() call! {}", err);
                }
            }
        }
    }
}