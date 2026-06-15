use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use omnisystem_cluster::*;
use std::time::Instant;

// Phase 7: Performance Benchmarking

fn benchmark_cluster_initialization(c: &mut Criterion) {
    c.bench_function("cluster_init", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let _manager = ClusterManager::new().await;
            });
    });
}

fn benchmark_voting_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("voting");

    for node_count in [3, 5, 7, 11, 21].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(node_count),
            node_count,
            |b, &node_count| {
                b.iter(|| {
                    let mut vm = voting::VotingManager::new(node_count).unwrap();
                    for i in 0..node_count {
                        vm.record_vote(&format!("node{}", i), i % 2 == 0)
                            .unwrap();
                    }
                    black_box(vm.has_majority());
                });
            },
        );
    }

    group.finish();
}

fn benchmark_state_machine_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_machine");

    group.bench_function("append_entry", |b| {
        b.iter(|| {
            let mut sm = state_machine::StateMachine::new().unwrap();
            let entry = state_machine::LogEntry {
                index: black_box(1),
                term: black_box(1),
                command: black_box(vec![1, 2, 3]),
            };
            sm.append_entry(entry).unwrap();
        });
    });

    group.bench_function("get_entries_100", |b| {
        let mut sm = state_machine::StateMachine::new().unwrap();
        for i in 0..100 {
            let entry = state_machine::LogEntry {
                index: i,
                term: 1,
                command: vec![i as u8],
            };
            sm.append_entry(entry).unwrap();
        }

        b.iter(|| {
            black_box(sm.get_entries(0, 100));
        });
    });

    group.bench_function("snapshot_creation", |b| {
        let mut sm = state_machine::StateMachine::new().unwrap();
        for i in 0..50 {
            let entry = state_machine::LogEntry {
                index: i,
                term: 1,
                command: vec![i as u8],
            };
            sm.append_entry(entry).unwrap();
        }

        b.iter(|| {
            sm.create_snapshot().unwrap();
        });
    });

    group.finish();
}

fn benchmark_leader_election(c: &mut Criterion) {
    let mut group = c.benchmark_group("leader_election");

    group.bench_function("state_transitions", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut mgr =
                    leader_election::LeaderElectionManager::new("node1".to_string()).unwrap();
                mgr.start_election().await.unwrap();
                mgr.become_leader().await.unwrap();
                mgr.revert_to_follower(Some("node2".to_string()))
                    .unwrap();
                black_box(mgr.state());
            });
    });

    group.bench_function("heartbeat_send", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut mgr =
                    leader_election::LeaderElectionManager::new("leader".to_string()).unwrap();
                mgr.become_leader().await.unwrap();
                mgr.send_heartbeat().unwrap();
            });
    });

    group.finish();
}

fn benchmark_membership_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership");

    group.bench_function("add_node_sequential", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mgr = ClusterManager::new().await.unwrap();
                let members = mgr.membership();
                for i in 0..10 {
                    members.add_node(&format!("node{}", i)).await.unwrap();
                }
            });
    });

    group.finish();
}

fn benchmark_consensus_engine(c: &mut Criterion) {
    c.bench_function("consensus_new_election_term", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let mut engine = consensus::ConsensusEngine::new().unwrap();
                engine.start_election().await.unwrap();
                black_box(engine.get_term());
            });
    });
}

criterion_group!(
    benches,
    benchmark_cluster_initialization,
    benchmark_voting_operations,
    benchmark_state_machine_operations,
    benchmark_leader_election,
    benchmark_membership_operations,
    benchmark_consensus_engine
);

criterion_main!(benches);
