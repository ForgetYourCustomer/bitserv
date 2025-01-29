#[cfg(test)]
mod tests {
    use bitserv::pubsub::{ChainEvent, Publisher};
    use lazy_static::lazy_static;
    use std::{sync::Arc, thread, time::Duration};
    use tokio::sync::Mutex;
    use zmq::{Context, Socket};

    struct TestContext {
        publisher: Publisher,
        subscriber: Socket,
    }

    impl TestContext {
        fn new() -> Self {
            println!("Creating test context...");
            // Create publisher
            let publisher = Publisher::new("tcp://*:5556").expect("Failed to create publisher");
            println!("Publisher created");

            // Create subscriber
            let context = Context::new();
            let subscriber = context
                .socket(zmq::SUB)
                .expect("Failed to create subscriber");
            println!("Subscriber created");

            subscriber
                .connect("tcp://localhost:5556")
                .expect("Failed to connect subscriber");
            println!("Subscriber connected");

            subscriber
                .set_subscribe(b"")
                .expect("Failed to set subscriber filter");
            println!("Subscriber filter set");

            // Give some time for the connection to establish
            thread::sleep(Duration::from_millis(500));
            println!("Connection setup complete");

            TestContext {
                publisher,
                subscriber,
            }
        }

        fn publish_event(&self, event: ChainEvent) {
            println!("Publishing event...");
            self.publisher
                .publish(event)
                .expect("Failed to publish event");
            println!("Event published");
        }

        fn receive_event(&self) -> ChainEvent {
            println!("Waiting to receive event...");
            // Receive the topic first
            let topic = self
                .subscriber
                .recv_string(0)
                .expect("Failed to receive topic")
                .expect("Invalid topic format");
            println!("Received topic: {}", topic);

            // Then receive the actual message
            let msg = self
                .subscriber
                .recv_string(0)
                .expect("Failed to receive message")
                .expect("Invalid message format");
            println!("Received message: {}", msg);

            serde_json::from_str(&msg).expect("Failed to parse event")
        }
    }

    lazy_static! {
        static ref TEST_CONTEXT: Arc<Mutex<TestContext>> = {
            println!("Initializing TEST_CONTEXT");
            let context = TestContext::new();
            println!("TEST_CONTEXT initialized");
            Arc::new(Mutex::new(context))
        };
    }

    #[tokio::test]
    async fn test_publish_new_transaction() {
        println!("Starting test_publish_new_transaction");
        println!("Acquiring context lock...");
        let context = TEST_CONTEXT.lock().await;
        println!("Context lock acquired");

        // Create and publish a new transaction event
        let event = ChainEvent::NewTransaction {
            txid: "abc123".to_string(),
            amount: 100000,
            confirmations: 1,
        };
        context.publish_event(event);

        // Receive and verify the event
        let received_event = context.receive_event();
        println!("Received event");

        loop {
            let event = ChainEvent::NewTransaction {
                txid: "abc123".to_string(),
                amount: 100000,
                confirmations: 1,
            };
            context.publish_event(event);
            println!("published event");
            thread::sleep(Duration::from_millis(2000));
        }

        match received_event {
            ChainEvent::NewTransaction {
                txid,
                amount,
                confirmations,
            } => {
                assert_eq!(txid, "abc123");
                assert_eq!(amount, 100000);
                assert_eq!(confirmations, 1);
            }
            _ => panic!("Received wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_publish_new_address() {
        println!("Starting test_publish_new_address");
        println!("Acquiring context lock...");
        let context = TEST_CONTEXT.lock().await;
        println!("Context lock acquired");

        // Create and publish a new address event
        let event = ChainEvent::NewAddress {
            address: "bc1qxxx...".to_string(),
        };
        context.publish_event(event);

        // Receive and verify the event
        let received_event = context.receive_event();
        println!("Received event");

        match received_event {
            ChainEvent::NewAddress { address } => {
                assert_eq!(address, "bc1qxxx...");
            }
            _ => panic!("Received wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_publish_new_deposits() {
        println!("Starting test_publish_new_deposits");
        println!("Acquiring context lock...");
        let context = TEST_CONTEXT.lock().await;
        println!("Context lock acquired");

        // Create and publish a new deposits event
        let deposits = vec![
            ("address1".to_string(), 50000, "txid1".to_string()),
            ("address2".to_string(), 75000, "txid2".to_string()),
        ];
        let event = ChainEvent::NewDeposits {
            deposits: deposits.clone(),
        };
        context.publish_event(event);

        // Receive and verify the event
        let received_event = context.receive_event();
        println!("Received event");

        match received_event {
            ChainEvent::NewDeposits {
                deposits: received_deposits,
            } => {
                assert_eq!(received_deposits, deposits);
            }
            _ => panic!("Received wrong event type"),
        }
    }
}
