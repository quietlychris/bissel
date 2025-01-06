use criterion::{criterion_group, criterion_main};
use meadow::prelude::*;
use rand::prelude::*;

/*
fn meadow_instantiation(c: &mut criterion::Criterion) {
    /*
    c.bench_function("create_host", |b| {
        b.iter(|| {
            let mut host = HostConfig::default().build().unwrap();
            host.start().unwrap();
        });
    });
    */

    c.bench_function("create_nodes", |b| {
        let mut host = HostConfig::default().build().unwrap();
        host.start().unwrap();
        b.iter(|| {
            let node = NodeConfig::<Tcp, usize>::new("SIMPLE_NODE")
                .topic("number")
                .build()
                .expect("Error in create_nodes benchmark");
            let _node = node.activate().unwrap();
        });
        host.stop().unwrap();
    });
}
*/

fn tcp_message_sending(c: &mut criterion::Criterion) {
    // Open a Host
    let mut host = HostConfig::default().build().unwrap();
    host.start().unwrap();
    // Create and activate a Node
    let node = NodeConfig::<Blocking, Tcp, usize>::new("number")
        .build()
        .unwrap();
    let node = node.activate().unwrap();
    let val = 1;

    c.bench_function("tcp_publish_usize", |b| {
        b.iter(|| {
            node.publish(val).unwrap();
        });
    });

    let tx = NodeConfig::<Blocking, Tcp, f32>::new("number")
        .build()
        .unwrap()
        .activate()
        .unwrap();
    let rx = NodeConfig::<Blocking, Tcp, f32>::new("number")
        .build()
        .unwrap()
        .activate()
        .unwrap();
    let val = 1.0f32;

    c.bench_function("tcp_publish_request_f32", |b| {
        // Open a Host

        b.iter(|| {
            tx.publish(val).unwrap();
            match rx.request() {
                Ok(_num) => (),
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            };
        });
    });

    static KB: usize = 1024;

    for size in [1, KB, 2 * KB, 4 * KB, 8 * KB].iter() {
        let bench_name = "msg_".to_owned() + &size.to_string();
        let mut rng = rand::thread_rng();
        // Create and activate a Node
        let tx = NodeConfig::<Blocking, Tcp, Vec<f32>>::new("number")
            .build()
            .unwrap()
            .activate()
            .unwrap();
        let rx = NodeConfig::<Blocking, Tcp, Vec<f32>>::new("number")
            .build()
            .unwrap()
            .activate()
            .unwrap();
        let mut nums: Vec<f32> = Vec::with_capacity(*size);
        for _ in 0..nums.len() {
            nums.push(rng.gen());
        }

        c.bench_function(&bench_name, |b| {
            b.iter(|| {
                tx.publish(nums.clone()).unwrap();
                let _result = rx.request().unwrap();
            });

            let result = rx.request().unwrap();
            assert_eq!(nums, result.data);
            // host.stop().unwrap();
        });
    }

    host.stop().unwrap();
}

criterion_group!(benches, tcp_message_sending);
criterion_main!(benches);

/*
/// Helper function for creating a simple network
fn create_meadow_triple() -> (Host, Node<Tcp, Active, f32>, Node<Tcp, Active, f32>) {
    let mut host = HostConfig::default().build().unwrap();
    host.start().unwrap();
    // Create and activate a Node
    let tx = NodeConfig::<Tcp, f32>::new("TX")
        .topic("number")
        .build()
        .unwrap()
        .activate()
        .unwrap();
    let rx = NodeConfig::<Tcp, f32>::new("RX")
        .topic("number")
        .build()
        .unwrap()
        .activate()
        .unwrap();
    (host, tx, rx)
}
*/
