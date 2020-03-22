extern crate libp2p;
extern crate tokio;

mod client {
    use futures::future::Future;
    use libp2p::core::identity;
    use libp2p::core::transport::Transport;
    use libp2p::core::upgrade::{self};
    use libp2p::core::Multiaddr;
    use libp2p::noise::{Keypair, NoiseConfig, X25519};
    use libp2p::tcp::TcpConfig;
    use tokio::{self, io};

    fn main() {
        env_logger::init();
        let client_id = identity::Keypair::generate_ed25519();

        let client_dh = Keypair::<X25519>::new().into_authentic(&client_id).unwrap();
        let client_transport = TcpConfig::new()
            .and_then(move |output, endpoint| {
                upgrade::apply(
                    output,
                    NoiseConfig::xx(client_dh),
                    endpoint,
                    upgrade::Version::V1,
                )
            })
            .timeout(std::time::Duration::from_secs(20));

        let server_address: Multiaddr = "/ip4/127.0.0.1/tcp/23456".parse().unwrap();
        let client = client_transport
            .dial(server_address.clone())
            .unwrap()
            .map_err(|e| panic!("client error: {}", e))
            .and_then(move |(_, server)| {
                io::write_all(server, "message2").and_then(|(client, _)| io::flush(client))
            })
            .map(|_| ())
            .map_err(|e| panic!("{:?}", e));

        tokio::run(client);
    }
}

mod server {
    use futures::future::Future;
    use futures::prelude::*;
    use libp2p::core::transport::{ListenerEvent, Transport};
    use libp2p::core::upgrade::{self};
    use libp2p::core::{identity, PeerId};
    use libp2p::noise::{Keypair, NoiseConfig, X25519};
    use libp2p::tcp::TcpConfig;
    use tokio::{self, io};

    fn main() {
        env_logger::init();

        let server_id = identity::Keypair::generate_ed25519();

        let server_dh = Keypair::<X25519>::new().into_authentic(&server_id).unwrap();
        let server_transport = TcpConfig::new().and_then(move |output, endpoint| {
            upgrade::apply(
                output,
                NoiseConfig::xx(server_dh),
                endpoint,
                upgrade::Version::V1,
            )
        });
        let peer_id = PeerId::from_public_key(server_id.public());
        dbg!(peer_id);
        let mut server = server_transport
            .listen_on("/ip4/127.0.0.1/tcp/23456".parse().unwrap())
            .unwrap();

        let server_address = server
            .by_ref()
            .wait()
            .next()
            .expect("some event")
            .expect("no error")
            .into_new_address()
            .expect("listen address");
        dbg!(server_address);
        let server = server
            .take(1)
            .filter_map(ListenerEvent::into_upgrade)
            .and_then(|client| client.0)
            .map_err(|e| panic!("server error: {}", e))
            .and_then(|(_, client)| {
                dbg!("server: reading message");
                io::read_to_end(client, Vec::new())
            })
            .for_each(move |msg| {
                dbg!(msg.1);
                Ok(())
            });

        tokio::run(server.map_err(|e| panic!("{:?}", e)).map(|_| ()));
    }
}

fn main() {
    println!("Hello, world!");
}
