use anyhow::Result;
use futures::StreamExt;   // provides Swarm::select_next_some
use libp2p::{
    gossipsub::{self, IdentTopic as Topic, MessageAuthenticity},
    identify, mdns,
    swarm::{NetworkBehaviour, SwarmEvent},
    PeerId, SwarmBuilder,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(NetworkBehaviour)]
pub struct HarnessBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns:      mdns::tokio::Behaviour,
    pub identify:  identify::Behaviour,
}

pub struct MeshNode {
    swarm:        libp2p::Swarm<HarnessBehaviour>,
    event_sender: mpsc::Sender<Vec<u8>>,
    topic:        Topic,
}

impl MeshNode {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Result<Self> {
        let key     = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(key.public());
        info!("[Mesh] Local peer ID: {}", peer_id);

        let gossipsub_cfg = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow::anyhow!("gossipsub config: {}", e))?;

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(key.clone()),
            gossipsub_cfg,
        ).map_err(|e| anyhow::anyhow!("gossipsub: {}", e))?;

        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
            .map_err(|e| anyhow::anyhow!("mdns: {}", e))?;

        let identify = identify::Behaviour::new(identify::Config::new(
            "/omniharness/1.0.0".to_string(),
            key.public(),
        ));

        let behaviour = HarnessBehaviour { gossipsub, mdns, identify };

        let swarm = SwarmBuilder::with_existing_identity(key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|_| behaviour)?
            .build();

        let topic = Topic::new("omniharness-events");

        Ok(Self { swarm, event_sender: tx, topic })
    }

    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic.clone(), data)
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("publish: {:?}", e))
    }

    pub fn peer_count(&self) -> usize {
        self.swarm.behaviour().gossipsub.all_peers().count()
    }

    pub async fn run(mut self) {
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&self.topic) {
            error!("[Mesh] Subscribe failed: {}", e);
            return;
        }
        if let Err(e) = self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()) {
            error!("[Mesh] Listen failed: {}", e);
            return;
        }

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(HarnessBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message { message, .. }
                )) => {
                    if self.event_sender.send(message.data).await.is_err() {
                        warn!("[Mesh] Channel closed — shutting down mesh.");
                        return;
                    }
                }
                SwarmEvent::Behaviour(HarnessBehaviourEvent::Mdns(
                    mdns::Event::Discovered(list)
                )) => {
                    for (peer_id, addr) in list {
                        info!("[Mesh] Discovered peer {} at {}", peer_id, addr);
                        self.swarm.dial(peer_id).ok();
                    }
                }
                SwarmEvent::Behaviour(HarnessBehaviourEvent::Mdns(
                    mdns::Event::Expired(list)
                )) => {
                    for (peer_id, _) in list {
                        info!("[Mesh] Peer {} expired.", peer_id);
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("[Mesh] Listening on {}", address);
                }
                _ => {}
            }
        }
    }
}
