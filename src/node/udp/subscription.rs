use crate::node::network_config::{Blocking, Nonblocking, Udp};
use crate::node::Node;
use crate::node::Subscription;
use crate::prelude::*;
use std::ops::Deref;

impl<T: Message + 'static> Node<Nonblocking, Udp, Subscription, T> {
    // Should actually return a <T>
    pub async fn get_subscribed_data(&self) -> Result<Msg<T>, crate::Error> {
        let data = self.subscription_data.lock().await.clone();
        if let Some(msg) = data {
            Ok(msg)
        } else {
            Err(Error::NoSubscriptionValue)
        }
    }
}

impl<T: Message + 'static> Node<Blocking, Udp, Subscription, T> {
    // Should actually return a <T>

    pub fn get_subscribed_data(&self) -> Result<Msg<T>, crate::Error> {
        let handle = match &self.rt_handle {
            Some(handle) => handle,
            None => return Err(Error::HandleAccess),
        };

        handle.block_on(async {
            let data = self.subscription_data.lock().await.clone();
            if let Some(msg) = data {
                Ok(msg)
            } else {
                Err(Error::NoSubscriptionValue)
            }
        })
    }
}
