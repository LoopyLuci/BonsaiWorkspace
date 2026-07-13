//! Comprehensive chaos engineering scenarios.
//!
//! 40+ pre-defined scenarios covering real-world failure modes.

use crate::error::Result;
use crate::schedule_generator::{ScheduleGenerator, ScheduleStrategy};

use super::{ChaosScenario, ImpactLevel, ScenarioCategory};

/// Get the Black Friday scenario: 10,000x normal load, network congestion, storage saturation.
pub fn scenario_black_friday() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 1, 1000, 7200);
    let schedule = gen.generate(15)?;

    let scenario = ChaosScenario::new(
        "Black Friday Traffic Surge".to_string(),
        "10,000x normal load spike with network congestion and storage saturation. \
         Simulates peak traffic shopping events (Black Friday, Prime Day). \
         Typical TTF: 120s, TTR: 600s".to_string(),
        ScenarioCategory::LoadTesting,
        ImpactLevel::Critical,
        120,
        600,
        schedule,
    )
    .with_incident("2019 Cyber Monday outages".to_string())
    .with_keywords(vec![
        "load".to_string(),
        "traffic".to_string(),
        "capacity".to_string(),
        "network".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Power Grid Failure scenario: rolling blackouts, partial power loss, UPS activation.
pub fn scenario_power_grid_failure() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 2, 1000, 1800);
    let schedule = gen.generate(12)?;

    let scenario = ChaosScenario::new(
        "Power Grid Failure".to_string(),
        "Rolling blackouts with partial power loss and UPS activation. \
         Includes phase transitions: normal → brown-out → blackout → recovery. \
         Typical TTF: 30s, TTR: 300s".to_string(),
        ScenarioCategory::PowerCooling,
        ImpactLevel::Critical,
        30,
        300,
        schedule,
    )
    .with_incident("2019 Argentina blackouts".to_string())
    .with_keywords(vec![
        "power".to_string(),
        "blackout".to_string(),
        "ups".to_string(),
        "infrastructure".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Data Center Fire scenario: cooling failure → thermal throttling → cascade shutdown.
pub fn scenario_data_center_fire() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 3, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Data Center Fire".to_string(),
        "Cooling system failure → thermal throttling → cascading shutdowns. \
         Simulates heat dissipation failure in data center. \
         Typical TTF: 60s, TTR: 900s".to_string(),
        ScenarioCategory::PowerCooling,
        ImpactLevel::Critical,
        60,
        900,
        schedule,
    )
    .with_incident("2018 OVH data center fire".to_string())
    .with_keywords(vec![
        "thermal".to_string(),
        "cooling".to_string(),
        "cascade".to_string(),
        "infrastructure".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Network Meltdown scenario: packet loss 1% → 50% → recovery.
pub fn scenario_network_meltdown() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 4, 1000, 1800);
    let schedule = gen.generate(14)?;

    let scenario = ChaosScenario::new(
        "Network Meltdown".to_string(),
        "Progressive packet loss: 1% → 10% → 50% → recovery. \
         Simulates routing/network hardware degradation. \
         Typical TTF: 90s, TTR: 180s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Critical,
        90,
        180,
        schedule,
    )
    .with_incident("2016 AWS us-east-1 outage".to_string())
    .with_keywords(vec![
        "network".to_string(),
        "packet_loss".to_string(),
        "routing".to_string(),
        "connectivity".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Storage Corruption scenario: bit flips in RAID data, RAID recovery under load.
pub fn scenario_storage_corruption() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 5, 1000, 2400);
    let schedule = gen.generate(16)?;

    let scenario = ChaosScenario::new(
        "Storage Corruption".to_string(),
        "Bit flips in RAID array triggers recovery rebuild under active I/O load. \
         Tests data integrity and rebuild performance. \
         Typical TTF: 120s, TTR: 1200s".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::Critical,
        120,
        1200,
        schedule,
    )
    .with_incident("2020 DigitalOcean storage incident".to_string())
    .with_keywords(vec![
        "storage".to_string(),
        "raid".to_string(),
        "corruption".to_string(),
        "data_integrity".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Byzantine Leader scenario: consensus node gives conflicting responses.
pub fn scenario_byzantine_leader() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 6, 1000, 1800);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Byzantine Leader".to_string(),
        "Consensus leader returns inconsistent state responses while claiming correctness. \
         Tests Byzantine Fault Tolerance and consensus mechanisms. \
         Typical TTF: 45s, TTR: 120s".to_string(),
        ScenarioCategory::Byzantine,
        ImpactLevel::High,
        45,
        120,
        schedule,
    )
    .with_incident("2014 Bitcoin consensus fork".to_string())
    .with_keywords(vec![
        "consensus".to_string(),
        "byzantine".to_string(),
        "distributed".to_string(),
        "correctness".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Zombie Apocalypse scenario: 50% of services become unresponsive (slow death).
pub fn scenario_zombie_apocalypse() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 7, 1000, 2400);
    let schedule = gen.generate(18)?;

    let scenario = ChaosScenario::new(
        "Zombie Apocalypse".to_string(),
        "50% of service instances become unresponsive but don't crash. \
         Simulates slow degradation, stuck threads, resource leaks. \
         Typical TTF: 180s, TTR: 600s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::Critical,
        180,
        600,
        schedule,
    )
    .with_incident("2018 Slack zombie processes".to_string())
    .with_keywords(vec![
        "degradation".to_string(),
        "unresponsive".to_string(),
        "cascading".to_string(),
        "resource_leak".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Cascading Restart scenario: service crash triggers dependent service crashes.
pub fn scenario_cascading_restart() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 8, 1000, 1200);
    let schedule = gen.generate(11)?;

    let scenario = ChaosScenario::new(
        "Cascading Restart".to_string(),
        "One service crashes, triggering cascading failures in dependent services. \
         Tests circuit breakers and failure isolation. \
         Typical TTF: 30s, TTR: 90s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::Critical,
        30,
        90,
        schedule,
    )
    .with_incident("2017 Gitlab database outage".to_string())
    .with_keywords(vec![
        "cascading".to_string(),
        "restart".to_string(),
        "dependencies".to_string(),
        "failure_isolation".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Memory Leak Under Load scenario: progressive memory exhaustion during high traffic.
pub fn scenario_memory_leak_under_load() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 9, 1000, 1800);
    let schedule = gen.generate(12)?;

    let scenario = ChaosScenario::new(
        "Memory Leak Under Load".to_string(),
        "Memory gradually leaks while handling peak traffic. \
         Tests OOM killers and graceful degradation. \
         Typical TTF: 150s, TTR: 120s".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::High,
        150,
        120,
        schedule,
    )
    .with_incident("2015 Node.js memory leak at Uber".to_string())
    .with_keywords(vec![
        "memory".to_string(),
        "leak".to_string(),
        "gc".to_string(),
        "resource_exhaustion".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Slow Query Cascade scenario: blocked database queries cause thread pool exhaustion.
pub fn scenario_slow_query_cascade() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 10, 1000, 1500);
    let schedule = gen.generate(13)?;

    let scenario = ChaosScenario::new(
        "Slow Query Cascade".to_string(),
        "Database slow queries cause thread pool exhaustion and cascading timeouts. \
         Tests query timeout mechanisms. \
         Typical TTF: 60s, TTR: 180s".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::High,
        60,
        180,
        schedule,
    )
    .with_incident("2019 Expedia database slowdown".to_string())
    .with_keywords(vec![
        "database".to_string(),
        "slow_query".to_string(),
        "thread_pool".to_string(),
        "timeout".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Network Partition scenario: split brain with divergent data.
pub fn scenario_network_partition() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 11, 1000, 1800);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Network Partition".to_string(),
        "Network splits into isolated partitions causing split-brain scenarios. \
         Tests quorum-based consensus and partition tolerance. \
         Typical TTF: 45s, TTR: 240s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Critical,
        45,
        240,
        schedule,
    )
    .with_incident("2012 AWS EBS failures".to_string())
    .with_keywords(vec![
        "network".to_string(),
        "partition".to_string(),
        "split_brain".to_string(),
        "consensus".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the DNS Chaos scenario: intermittent DNS resolution failures and latency spikes.
pub fn scenario_dns_chaos() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 12, 1000, 1200);
    let schedule = gen.generate(14)?;

    let scenario = ChaosScenario::new(
        "DNS Chaos".to_string(),
        "Intermittent DNS failures mixed with latency spikes (10ms → 5000ms). \
         Tests DNS caching and failover mechanisms. \
         Typical TTF: 80s, TTR: 60s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::High,
        80,
        60,
        schedule,
    )
    .with_incident("2018 GitHub DNS incident".to_string())
    .with_keywords(vec![
        "dns".to_string(),
        "network".to_string(),
        "resolution".to_string(),
        "latency".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Cache Stampede scenario: cache expiration during traffic surge causes backend collapse.
pub fn scenario_cache_stampede() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 13, 1000, 900);
    let schedule = gen.generate(8)?;

    let scenario = ChaosScenario::new(
        "Cache Stampede".to_string(),
        "Cache mass expiration coincides with traffic surge causing backend overload. \
         Tests cache refresh strategies and thundering herd mitigation. \
         Typical TTF: 45s, TTR: 120s".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::High,
        45,
        120,
        schedule,
    )
    .with_incident("2014 Instagram cache failure".to_string())
    .with_keywords(vec![
        "cache".to_string(),
        "thundering_herd".to_string(),
        "stampede".to_string(),
        "backend".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Connection Pool Exhaustion scenario: database connections leak and run out.
pub fn scenario_connection_pool_exhaustion() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 14, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Connection Pool Exhaustion".to_string(),
        "Database connections leak without being returned causing pool exhaustion. \
         Tests connection timeout and cleanup mechanisms. \
         Typical TTF: 90s, TTR: 150s".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::High,
        90,
        150,
        schedule,
    )
    .with_incident("2018 RDS connection leak at Notion".to_string())
    .with_keywords(vec![
        "database".to_string(),
        "connection".to_string(),
        "pool".to_string(),
        "leak".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Silent Data Loss scenario: writes return success but data isn't persisted.
pub fn scenario_silent_data_loss() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 15, 1000, 1500);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Silent Data Loss".to_string(),
        "Write operations return success but data is silently lost due to storage failure. \
         Tests write consistency and durability guarantees. \
         Typical TTF: 30s, TTR: 0s (permanent)".to_string(),
        ScenarioCategory::Silent,
        ImpactLevel::Critical,
        30,
        0,
        schedule,
    )
    .with_incident("2013 Microsoft Azure storage loss".to_string())
    .with_keywords(vec![
        "data_loss".to_string(),
        "silent".to_string(),
        "durability".to_string(),
        "storage".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the CPU Throttling scenario: sustained CPU overload causing automatic throttling.
pub fn scenario_cpu_throttling() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 16, 1000, 1800);
    let schedule = gen.generate(12)?;

    let scenario = ChaosScenario::new(
        "CPU Throttling".to_string(),
        "Sustained CPU overload triggers frequency scaling and performance degradation. \
         Tests vertical scaling and load shedding. \
         Typical TTF: 120s, TTR: 180s".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::Medium,
        120,
        180,
        schedule,
    )
    .with_incident("2015 AWS on-demand CPU throttling".to_string())
    .with_keywords(vec![
        "cpu".to_string(),
        "throttling".to_string(),
        "performance".to_string(),
        "scaling".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the File Descriptor Exhaustion scenario: too many open files causes service failure.
pub fn scenario_file_descriptor_exhaustion() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 17, 1000, 1200);
    let schedule = gen.generate(11)?;

    let scenario = ChaosScenario::new(
        "File Descriptor Exhaustion".to_string(),
        "Too many open files (connections, files) hit system limits. \
         Tests ulimit enforcement and resource cleanup. \
         Typical TTF: 100s, TTR: 120s".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::High,
        100,
        120,
        schedule,
    )
    .with_incident("2016 Twilio FD limit hit".to_string())
    .with_keywords(vec![
        "file_descriptors".to_string(),
        "ulimit".to_string(),
        "resource_exhaustion".to_string(),
        "system_limits".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Latency Amplification scenario: small latencies cascade causing exponential slowdown.
pub fn scenario_latency_amplification() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 18, 1000, 1500);
    let schedule = gen.generate(13)?;

    let scenario = ChaosScenario::new(
        "Latency Amplification".to_string(),
        "Small network latency (50ms) cascades through microservices causing exponential slowdown. \
         Tests timeout configuration and service isolation. \
         Typical TTF: 60s, TTR: 120s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::High,
        60,
        120,
        schedule,
    )
    .with_incident("2014 Microservices at Amazon".to_string())
    .with_keywords(vec![
        "latency".to_string(),
        "cascade".to_string(),
        "microservices".to_string(),
        "timeout".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Partial Request Failure scenario: requests partially succeed causing inconsistency.
pub fn scenario_partial_request_failure() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 19, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Partial Request Failure".to_string(),
        "Some shards/replicas fail causing partial request success and inconsistent state. \
         Tests idempotency and transaction handling. \
         Typical TTF: 40s, TTR: 100s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::High,
        40,
        100,
        schedule,
    )
    .with_incident("2016 Uber database consistency issue".to_string())
    .with_keywords(vec![
        "partial_failure".to_string(),
        "consistency".to_string(),
        "idempotency".to_string(),
        "sharding".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Authentication Service Failure scenario: auth service down causes cascading failures.
pub fn scenario_auth_service_failure() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 20, 1000, 1200);
    let schedule = gen.generate(8)?;

    let scenario = ChaosScenario::new(
        "Authentication Service Failure".to_string(),
        "Central auth service becomes unavailable blocking all operations. \
         Tests cache-aside authentication and bypass mechanisms. \
         Typical TTF: 5s, TTR: 60s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::Critical,
        5,
        60,
        schedule,
    )
    .with_incident("2020 Auth0 outage".to_string())
    .with_keywords(vec![
        "auth".to_string(),
        "cascading".to_string(),
        "availability".to_string(),
        "centralized_service".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Metric Pipeline Overflow scenario: monitoring system gets overloaded causing metrics loss.
pub fn scenario_metric_pipeline_overflow() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 21, 1000, 900);
    let schedule = gen.generate(7)?;

    let scenario = ChaosScenario::new(
        "Metric Pipeline Overflow".to_string(),
        "Monitoring pipeline cannot keep up with metric volume causing loss of observability. \
         Tests monitoring resilience and backpressure. \
         Typical TTF: 50s, TTR: 60s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::Medium,
        50,
        60,
        schedule,
    )
    .with_incident("2019 Datadog ingestion failure".to_string())
    .with_keywords(vec![
        "monitoring".to_string(),
        "metrics".to_string(),
        "observability".to_string(),
        "backpressure".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Certificate Expiration scenario: SSL/TLS certificates expire causing connectivity failures.
pub fn scenario_certificate_expiration() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 22, 1000, 1200);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Certificate Expiration".to_string(),
        "SSL/TLS certificates expire causing all HTTPS connections to fail. \
         Tests certificate renewal and rollover processes. \
         Typical TTF: 0s (immediate), TTR: 300s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Critical,
        0,
        300,
        schedule,
    )
    .with_incident("2011 DigiNotar CA compromise".to_string())
    .with_keywords(vec![
        "tls".to_string(),
        "certificate".to_string(),
        "security".to_string(),
        "renewal".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Load Balancer Failure scenario: load balancer crashes or becomes unhealthy.
pub fn scenario_load_balancer_failure() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 23, 1000, 1500);
    let schedule = gen.generate(11)?;

    let scenario = ChaosScenario::new(
        "Load Balancer Failure".to_string(),
        "Primary load balancer fails forcing failover to secondary. \
         Tests failover speed and health check mechanisms. \
         Typical TTF: 10s (health detection), TTR: 120s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Critical,
        10,
        120,
        schedule,
    )
    .with_incident("2016 AWS ELB failures".to_string())
    .with_keywords(vec![
        "load_balancer".to_string(),
        "failover".to_string(),
        "ha".to_string(),
        "health_check".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Request Amplification Attack scenario: small requests cause large responses.
pub fn scenario_request_amplification() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 24, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Request Amplification".to_string(),
        "Requests trigger unexpectedly large responses overwhelming bandwidth. \
         Tests response size limits and bandwidth throttling. \
         Typical TTF: 30s, TTR: 90s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::High,
        30,
        90,
        schedule,
    )
    .with_incident("2015 GitHub DDoS incident".to_string())
    .with_keywords(vec![
        "ddos".to_string(),
        "amplification".to_string(),
        "bandwidth".to_string(),
        "network".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Garbage Collection Pause scenario: long GC pauses cause service timeouts.
pub fn scenario_gc_pause() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 25, 1000, 1800);
    let schedule = gen.generate(12)?;

    let scenario = ChaosScenario::new(
        "Garbage Collection Pause".to_string(),
        "Long garbage collection pauses (500ms-2s) cause request timeouts and cascading failures. \
         Tests stop-the-world pause tolerance. \
         Typical TTF: 45s, TTR: 60s".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::High,
        45,
        60,
        schedule,
    )
    .with_incident("2016 Java GC at Twitter".to_string())
    .with_keywords(vec![
        "gc".to_string(),
        "pause".to_string(),
        "memory".to_string(),
        "latency".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Replication Lag scenario: master-slave replication lag causes read inconsistencies.
pub fn scenario_replication_lag() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 26, 1000, 1500);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Replication Lag".to_string(),
        "Database replication lag (seconds to minutes) causes reads of stale data. \
         Tests read-your-own-writes consistency. \
         Typical TTF: 0s (immediate), TTR: 30-300s depending on catchup".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::High,
        0,
        300,
        schedule,
    )
    .with_incident("2016 MySQL replication at Reddit".to_string())
    .with_keywords(vec![
        "replication".to_string(),
        "consistency".to_string(),
        "database".to_string(),
        "lag".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Disk I/O Saturation scenario: I/O operations saturate causing slowdown.
pub fn scenario_disk_io_saturation() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 27, 1000, 1800);
    let schedule = gen.generate(13)?;

    let scenario = ChaosScenario::new(
        "Disk I/O Saturation".to_string(),
        "Disk I/O operations saturate causing request latency to spike 100x. \
         Tests I/O queue depth and scheduling. \
         Typical TTF: 120s, TTR: 180s".to_string(),
        ScenarioCategory::Storage,
        ImpactLevel::High,
        120,
        180,
        schedule,
    )
    .with_incident("2013 AWS EBS performance issues".to_string())
    .with_keywords(vec![
        "io".to_string(),
        "disk".to_string(),
        "saturation".to_string(),
        "latency".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Kernel OOM Killer scenario: OOM killer randomly kills processes.
pub fn scenario_kernel_oom_killer() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 28, 1000, 1200);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Kernel OOM Killer".to_string(),
        "System memory exhaustion triggers OOM killer which kills random processes. \
         Tests process recovery and graceful shutdown. \
         Typical TTF: 150s (memory buildup), TTR: instant (process respawn)".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::Critical,
        150,
        5,
        schedule,
    )
    .with_incident("2017 Memory pressure at Kubernetes".to_string())
    .with_keywords(vec![
        "oom".to_string(),
        "memory".to_string(),
        "killer".to_string(),
        "process".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Message Queue Overflow scenario: message queue backlog grows exponentially.
pub fn scenario_message_queue_overflow() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 29, 1000, 1500);
    let schedule = gen.generate(11)?;

    let scenario = ChaosScenario::new(
        "Message Queue Overflow".to_string(),
        "Message queue consumption rate drops causing exponential backlog growth. \
         Tests queue size limits and producer blocking. \
         Typical TTF: 120s, TTR: 300s (catch-up)".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::High,
        120,
        300,
        schedule,
    )
    .with_incident("2015 RabbitMQ at LinkedIn".to_string())
    .with_keywords(vec![
        "queue".to_string(),
        "backlog".to_string(),
        "cascading".to_string(),
        "messaging".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Clock Skew scenario: system clock jumps backwards/forwards confusing timestamps.
pub fn scenario_clock_skew() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 30, 1000, 900);
    let schedule = gen.generate(8)?;

    let scenario = ChaosScenario::new(
        "Clock Skew".to_string(),
        "System clock jumps backward or forward confusing timestamp-based ordering. \
         Tests NTP corrections and monotonic clock usage. \
         Typical TTF: 0s (immediate), TTR: 60s (NTP correction)".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::High,
        0,
        60,
        schedule,
    )
    .with_incident("2012 AWS clock skew".to_string())
    .with_keywords(vec![
        "clock".to_string(),
        "time".to_string(),
        "ntp".to_string(),
        "ordering".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Resource Starvation scenario: one process starves others of resources.
pub fn scenario_resource_starvation() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 31, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Resource Starvation".to_string(),
        "One long-running operation starves others of CPU, memory, or I/O. \
         Tests fair scheduling and resource isolation. \
         Typical TTF: 60s, TTR: 120s".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::High,
        60,
        120,
        schedule,
    )
    .with_incident("2014 cgroup issues at Google".to_string())
    .with_keywords(vec![
        "starvation".to_string(),
        "scheduling".to_string(),
        "fairness".to_string(),
        "resource".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Zombie Process Leak scenario: child processes become zombies accumulating.
pub fn scenario_zombie_process_leak() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 32, 1000, 1800);
    let schedule = gen.generate(12)?;

    let scenario = ChaosScenario::new(
        "Zombie Process Leak".to_string(),
        "Child processes become zombies due to missing wait() calls accumulating over time. \
         Tests process cleanup and signal handlers. \
         Typical TTF: 180s (accumulation), TTR: 60s (reap)".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::Medium,
        180,
        60,
        schedule,
    )
    .with_incident("2018 Zombie processes at Kubernetes".to_string())
    .with_keywords(vec![
        "process".to_string(),
        "zombie".to_string(),
        "leak".to_string(),
        "signal".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Memory Fragmentation scenario: memory becomes fragmented preventing allocation.
pub fn scenario_memory_fragmentation() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 33, 1000, 2400);
    let schedule = gen.generate(14)?;

    let scenario = ChaosScenario::new(
        "Memory Fragmentation".to_string(),
        "Memory fragmentation accumulates preventing large allocations despite total free space. \
         Tests memory allocator fragmentation resilience. \
         Typical TTF: 300s (fragmentation buildup), TTR: 60s (defrag/restart)".to_string(),
        ScenarioCategory::Compute,
        ImpactLevel::Medium,
        300,
        60,
        schedule,
    )
    .with_incident("2019 Memory fragmentation at Postgres".to_string())
    .with_keywords(vec![
        "memory".to_string(),
        "fragmentation".to_string(),
        "allocation".to_string(),
        "heap".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Request Timeout Cascade scenario: one timeout causes cascading timeouts.
pub fn scenario_request_timeout_cascade() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 34, 1000, 1200);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Request Timeout Cascade".to_string(),
        "Initial request timeout causes cascading timeouts in dependent services. \
         Tests timeout configuration and circuit breaker patterns. \
         Typical TTF: 30s, TTR: 90s".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::High,
        30,
        90,
        schedule,
    )
    .with_incident("2015 Microservices timeout at Amazon".to_string())
    .with_keywords(vec![
        "timeout".to_string(),
        "cascade".to_string(),
        "circuit_breaker".to_string(),
        "latency".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Quorum Loss scenario: losing majority in distributed system.
pub fn scenario_quorum_loss() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 35, 1000, 1500);
    let schedule = gen.generate(10)?;

    let scenario = ChaosScenario::new(
        "Quorum Loss".to_string(),
        "Majority of nodes fail preventing quorum for consensus decisions. \
         Tests quorum-based fault tolerance limits. \
         Typical TTF: 10s (detection), TTR: 300s (recovery)".to_string(),
        ScenarioCategory::Byzantine,
        ImpactLevel::Critical,
        10,
        300,
        schedule,
    )
    .with_incident("2013 MongoDB replica set quorum loss".to_string())
    .with_keywords(vec![
        "quorum".to_string(),
        "consensus".to_string(),
        "distributed".to_string(),
        "availability".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Cache Invalidation nightmare scenario: cache and reality diverge.
pub fn scenario_cache_invalidation_divergence() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 36, 1000, 1800);
    let schedule = gen.generate(13)?;

    let scenario = ChaosScenario::new(
        "Cache Invalidation Divergence".to_string(),
        "Cache invalidation messages lost causing cache to diverge from reality. \
         Tests cache consistency mechanisms and TTL enforcement. \
         Typical TTF: 0s (immediate), TTR: 300s (TTL expiry)".to_string(),
        ScenarioCategory::Silent,
        ImpactLevel::High,
        0,
        300,
        schedule,
    )
    .with_incident("2014 Redis cache invalidation at Airbnb".to_string())
    .with_keywords(vec![
        "cache".to_string(),
        "consistency".to_string(),
        "invalidation".to_string(),
        "divergence".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Timeout Underestimation scenario: P99 latencies exceed conservative timeouts.
pub fn scenario_timeout_underestimation() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 37, 1000, 1200);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Timeout Underestimation".to_string(),
        "Timeout values set lower than P99 latencies causing false timeout failures. \
         Tests timeout tuning and percentile-based configuration. \
         Typical TTF: constant (improper config), TTR: config change".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Medium,
        0,
        60,
        schedule,
    )
    .with_incident("2016 Timeout misconfiguration at Uber".to_string())
    .with_keywords(vec![
        "timeout".to_string(),
        "latency".to_string(),
        "configuration".to_string(),
        "p99".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Transaction Rollback Cascade scenario: database rollback cascades through system.
pub fn scenario_transaction_rollback_cascade() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 38, 1000, 1500);
    let schedule = gen.generate(11)?;

    let scenario = ChaosScenario::new(
        "Transaction Rollback Cascade".to_string(),
        "Database transaction rollback causes cascading failures in dependent components. \
         Tests transaction isolation and idempotency. \
         Typical TTF: 60s, TTR: 180s".to_string(),
        ScenarioCategory::Cascading,
        ImpactLevel::High,
        60,
        180,
        schedule,
    )
    .with_incident("2018 Transaction isolation at Stripe".to_string())
    .with_keywords(vec![
        "transaction".to_string(),
        "rollback".to_string(),
        "database".to_string(),
        "cascade".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Dependency Version Mismatch scenario: deployed version differs from loaded version.
pub fn scenario_dependency_version_mismatch() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 39, 1000, 900);
    let schedule = gen.generate(8)?;

    let scenario = ChaosScenario::new(
        "Dependency Version Mismatch".to_string(),
        "Deployed service version differs from loaded library version causing runtime errors. \
         Tests library compatibility and semantic versioning. \
         Typical TTF: 0s (immediate), TTR: 600s (redeploy)".to_string(),
        ScenarioCategory::Silent,
        ImpactLevel::High,
        0,
        600,
        schedule,
    )
    .with_incident("2017 Dependency hell at npm".to_string())
    .with_keywords(vec![
        "dependency".to_string(),
        "versioning".to_string(),
        "compatibility".to_string(),
        "deployment".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}

/// Get the Exponential Backoff Failure scenario: backoff exceeds test timeout causing failure.
pub fn scenario_exponential_backoff_failure() -> Result<ChaosScenario> {
    let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 40, 1000, 1200);
    let schedule = gen.generate(9)?;

    let scenario = ChaosScenario::new(
        "Exponential Backoff Failure".to_string(),
        "Exponential backoff algorithm delays recovery beyond acceptable timeframe. \
         Tests backoff configuration and max-retry policies. \
         Typical TTF: 30s, TTR: depends on backoff (could be 300s+)".to_string(),
        ScenarioCategory::Network,
        ImpactLevel::Medium,
        30,
        300,
        schedule,
    )
    .with_incident("2016 Backoff misconfiguration at GitHub".to_string())
    .with_keywords(vec![
        "backoff".to_string(),
        "retry".to_string(),
        "timeout".to_string(),
        "recovery".to_string(),
    ]);

    scenario.validate()?;
    Ok(scenario)
}
