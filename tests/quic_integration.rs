#![cfg(feature = "quic")]
use meadow::*;

mod common;
use common::*;

use std::thread;
use std::time::Duration;

#[cfg(feature = "quic")]
use std::sync::Once;

#[cfg(feature = "quic")]
use meadow::host::quic::generate_certs;

#[cfg(feature = "quic")]
static INIT: Once = Once::new();

#[cfg(feature = "quic")]
pub fn initialize() {
    use meadow::host::quic::QuicCertGenConfig;

    INIT.call_once(|| {
        generate_certs(QuicCertGenConfig::default());
    });
}

#[cfg(feature = "quic")]
type N = Quic;

#[cfg(feature = "quic")]
#[test]
fn integrate_host_and_single_node_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();
    println!("Host should be running in the background");

    // Get the host up and running
    let node: Node<N, Idle, Pose> = NodeConfig::new("pose").build().unwrap();
    let node = node.activate().unwrap();

    for i in 0..5 {
        // Could get this by reading a GPS, for example
        let pose = Pose {
            x: i as f32,
            y: i as f32,
        };

        node.publish(pose.clone()).unwrap();
        thread::sleep(Duration::from_millis(10));
        let result = node.request().unwrap();
        println!("Got position: {:?}", result);

        assert_eq!(pose, result.data);
    }
}

#[cfg(feature = "quic")]
#[test]
fn request_non_existent_topic_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();
    println!("Host should be running in the background");

    // Get the host up and running
    let node: Node<N, Idle, Pose> = NodeConfig::new("doesnt_exist").build().unwrap();
    let node = node.activate().unwrap();

    // Requesting a topic that doesn't exist should return a recoverable error
    for i in 0..5 {
        println!("on loop: {}", i);
        let result = node.request();
        dbg!(&result);
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(feature = "quic")]
#[test]
fn node_send_options_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();

    // Get the host up and running
    let node_a = NodeConfig::<N, Option<f32>>::new("pose")
        .build()
        .unwrap()
        .activate()
        .unwrap();
    let node_b = NodeConfig::<N, Option<f32>>::new("pose")
        .build()
        .unwrap()
        .activate()
        .unwrap();

    // Send Option with `Some(value)`
    node_a.publish(Some(1.0)).unwrap();
    let result = node_b.request().unwrap();
    dbg!(&result);
    assert_eq!(result.data.unwrap(), 1.0);

    // Send option with `None`
    node_a.publish(None).unwrap();
    let result = node_b.request();
    dbg!(&result);
    assert_eq!(result.unwrap().data, None);
}

#[cfg(feature = "quic")]
#[test]
fn subscription_usize_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();

    // Get the host up and running
    let writer = NodeConfig::<N, usize>::new("subscription")
        .build()
        .unwrap()
        .activate()
        .unwrap();

    // Create a subscription node with a query rate of 100 Hz
    let reader = writer
        .cfg
        .clone()
        .build()
        .unwrap()
        .subscribe(Duration::from_millis(10))
        .unwrap();

    for i in 0..5 {
        let test_value = i as usize;
        writer.publish(test_value).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        // let result = reader.get_subscribed_data();
        match reader.get_subscribed_data() {
            Ok(result) => assert_eq!(test_value, result.data),
            Err(e) => println!("{:?}", e),
        }
        // dbg!(result);
    }
}

#[cfg(feature = "quic")]
#[test]
#[should_panic]
fn no_subscribed_value_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();

    // Create a subscription node with a query rate of 10 Hz
    let reader = NodeConfig::<N, usize>::new("subscription")
        .build()
        .unwrap()
        .subscribe(Duration::from_millis(100))
        .unwrap();

    // Unwrapping on an error should lead to panic
    let _result: usize = reader.get_subscribed_data().unwrap().data;
}

#[cfg(feature = "quic")]
#[test]
fn topics_list_quic() {
    let mut host: Host = HostConfig::default().with_udp_config(None).build().unwrap();
    host.start().unwrap();
    println!("Host should be running in the background");

    // Get the host up and running
    let topics: Vec<String> = ["a", "b", "c", "d", "e", "f"]
        .iter()
        .map(|x| x.to_string())
        .collect();
    dbg!(&topics);
    let mut nodes = Vec::with_capacity(topics.len());
    for topic in topics.clone() {
        let node: Node<N, Idle, usize> = NodeConfig::new(topic).build().unwrap();
        let node = node.activate().unwrap();
        nodes.push(node);
    }

    for i in 0..topics.len() {
        nodes[i].publish(i).unwrap();
        thread::sleep(Duration::from_millis(1));
        assert_eq!(host.topics(), nodes[i].topics().unwrap().data);
        let t = if i == 0 {
            vec![topics[i].to_string()]
        } else {
            let mut t = topics[0..i + 1]
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>();
            t.sort();
            t
        };
        let mut nt = nodes[i].topics().unwrap().data;
        nt.sort();
        assert_eq!(t, nt);
    }
}
