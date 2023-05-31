use std::{error::Error, time::Duration};

use libp2p::{
	core::upgrade,
	dns::TokioDnsConfig,
	futures::StreamExt,
	identity, mdns,
	mdns::Config,
	noise,
	swarm::{SwarmBuilder, SwarmEvent},
	tcp::{tokio::Transport, Config as TcpConfig},
	yamux, PeerId, Transport as TransportTrait,
};

use crate::mdns::tokio::Behaviour;

const TWO_HOURS: Duration = Duration::from_secs(60 * 60 * 2);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	// Generate a random keypair
	let local_key = identity::Keypair::generate_ed25519();
	let local_peer_id = PeerId::from(local_key.public());

	// Create a TCP Transport with Tokio
	let transport = {
		let dns_tcp =
			TokioDnsConfig::system(Transport::new(TcpConfig::new().nodelay(true))).unwrap();

		let tcp = Transport::new(TcpConfig::default().nodelay(true));
		dns_tcp.or_transport(tcp)
	};

	let transport = transport
		.upgrade(upgrade::Version::V1)
		.authenticate(noise::Config::new(&local_key).unwrap())
		.multiplex(yamux::Config::default())
		.timeout(TWO_HOURS)
		.boxed();

	// // Create a Kademlia behaviour
	// let kad_store = MemoryStore::new(local_peer_id.clone());
	// let mut kad_cfg = KademliaConfig::default();
	// kad_cfg.set_protocol_names(vec![Cow::Borrowed(b"/libp2p/kad/1.0.0")]);
	// let kademlia = Kademlia::with_config(local_peer_id.clone(), kad_store, kad_cfg);

	// Create an mDNS behaviour
	let mdns: mdns::Behaviour<_> = Behaviour::new(Config::default(), local_peer_id).unwrap();

	// Compose all the behaviours into a "Swarm"
	let mut swarm = SwarmBuilder::with_tokio_executor(transport, mdns, local_peer_id).build();

	// Listen on all interfaces and whatever port the OS assigns
	swarm
		.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
		.unwrap();

	// Start the Swarm
	while let Some(event) = swarm.next().await {
		match event {
			SwarmEvent::NewListenAddr { address, .. } => {
				println!("Listening on {}", address);
			}
			SwarmEvent::Behaviour(mdns::Event::Discovered(list)) => {
				for (peer, _) in list {
					println!("Discovered {}", peer);
				}
			}
			_ => {} // handle the events you're interested in
		}
	}

	Ok(())
}
