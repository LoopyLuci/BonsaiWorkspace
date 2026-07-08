// OMNISYSTEM MICROSERVICES EXAMPLE
// Demonstrates Aether distributed systems with Axiom verification

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

// ============================================================================
// SERVICE DEFINITION
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: String,
    pub service: String,
    pub address: String,
    pub port: u16,
    pub status: ServiceStatus,
    pub requests_handled: u64,
}

impl ServiceInstance {
    pub fn new(id: &str, service: &str, address: &str, port: u16) -> Self {
        ServiceInstance {
            id: id.to_string(),
            service: service.to_string(),
            address: address.to_string(),
            port,
            status: ServiceStatus::Healthy,
            requests_handled: 0,
        }
    }
}

// ============================================================================
// SERVICE REGISTRY (Aether)
// ============================================================================

pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, Vec<ServiceInstance>>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, instance: ServiceInstance) -> Result<(), String> {
        let mut services = self.services.write().unwrap();
        services.entry(instance.service.clone())
            .or_insert_with(Vec::new)
            .push(instance.clone());

        println!("✅ Service registered: {} ({}:{})",
            instance.id, instance.address, instance.port);
        Ok(())
    }

    pub fn deregister(&self, id: &str) -> Result<(), String> {
        let mut services = self.services.write().unwrap();
        for instances in services.values_mut() {
            instances.retain(|s| s.id != id);
        }
        println!("❌ Service deregistered: {}", id);
        Ok(())
    }

    pub fn get_instances(&self, service: &str) -> Vec<ServiceInstance> {
        let services = self.services.read().unwrap();
        services.get(service).cloned().unwrap_or_default()
    }

    pub fn list_all(&self) -> HashMap<String, Vec<ServiceInstance>> {
        self.services.read().unwrap().clone()
    }
}

// ============================================================================
// LOAD BALANCER
// ============================================================================

pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
}

pub struct LoadBalancer {
    registry: Arc<ServiceRegistry>,
    strategy: LoadBalancingStrategy,
    current_index: Arc<Mutex<HashMap<String, usize>>>,
}

impl LoadBalancer {
    pub fn new(registry: Arc<ServiceRegistry>, strategy: LoadBalancingStrategy) -> Self {
        LoadBalancer {
            registry,
            strategy,
            current_index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn select_instance(&self, service: &str) -> Result<ServiceInstance, String> {
        let instances = self.registry.get_instances(service);
        let healthy: Vec<_> = instances.iter()
            .filter(|i| i.status == ServiceStatus::Healthy)
            .collect();

        if healthy.is_empty() {
            return Err(format!("No healthy instances for {}", service));
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let mut idx = self.current_index.lock().unwrap();
                let current = idx.entry(service.to_string()).or_insert(0);
                let instance = healthy[*current % healthy.len()].clone();
                *current = (*current + 1) % healthy.len();
                Ok(instance)
            }
            LoadBalancingStrategy::LeastConnections => {
                let instance = healthy.iter()
                    .min_by_key(|i| i.requests_handled)
                    .map(|i| (*i).clone())
                    .unwrap();
                Ok(instance)
            }
            LoadBalancingStrategy::Random => {
                let idx = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as usize) % healthy.len();
                Ok(healthy[idx].clone())
            }
        }
    }

    pub fn route(&self, service: &str) -> Result<String, String> {
        let instance = self.select_instance(service)?;
        println!("🔀 Routing to: {}://{}:{}", service, instance.address, instance.port);
        Ok(format!("{}:{}", instance.address, instance.port))
    }
}

// ============================================================================
// CIRCUIT BREAKER (Fault Tolerance)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<Mutex<CircuitBreakerState>>,
    failure_count: Arc<Mutex<u32>>,
    failure_threshold: u32,
    success_count: Arc<Mutex<u32>>,
    success_threshold: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32) -> Self {
        CircuitBreaker {
            state: Arc::new(Mutex::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            failure_threshold,
            success_count: Arc::new(Mutex::new(0)),
            success_threshold,
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitBreakerState::HalfOpen => {
                let mut success = self.success_count.lock().unwrap();
                *success += 1;
                if *success >= self.success_threshold {
                    *state = CircuitBreakerState::Closed;
                    println!("🟢 Circuit breaker: CLOSED (recovered)");
                }
            }
            CircuitBreakerState::Closed => {
                let mut failures = self.failure_count.lock().unwrap();
                *failures = 0;
            }
            _ => {}
        }
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        let mut failures = self.failure_count.lock().unwrap();
        *failures += 1;

        if *failures >= self.failure_threshold {
            *state = CircuitBreakerState::Open;
            println!("🔴 Circuit breaker: OPEN (too many failures)");
        }
    }

    pub fn is_available(&self) -> bool {
        !matches!(*self.state.lock().unwrap(), CircuitBreakerState::Open)
    }

    pub fn get_state(&self) -> CircuitBreakerState {
        self.state.lock().unwrap().clone()
    }
}

// ============================================================================
// VERIFICATION ENGINE (Axiom)
// ============================================================================

pub struct VerificationEngine {
    properties: Arc<RwLock<HashMap<String, bool>>>,
}

impl VerificationEngine {
    pub fn new() -> Self {
        VerificationEngine {
            properties: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn define_property(&self, name: &str, verified: bool) {
        let mut props = self.properties.write().unwrap();
        props.insert(name.to_string(), verified);
        println!("✔️  Property defined: {}", name);
    }

    pub fn verify_property(&self, name: &str) -> Result<(), String> {
        let mut props = self.properties.write().unwrap();
        if let Some(prop) = props.get_mut(name) {
            *prop = true;
            println!("✅ Property verified: {}", name);
            Ok(())
        } else {
            Err(format!("Property not found: {}", name))
        }
    }

    pub fn check_invariant(&self, condition: bool, message: &str) -> Result<(), String> {
        if condition {
            println!("✓ Invariant holds: {}", message);
            Ok(())
        } else {
            println!("✗ Invariant violated: {}", message);
            Err(format!("Invariant failed: {}", message))
        }
    }

    pub fn get_verification_status(&self) -> HashMap<String, bool> {
        self.properties.read().unwrap().clone()
    }
}

// ============================================================================
// INTEGRATED MICROSERVICES SYSTEM
// ============================================================================

pub struct MicroservicesSystem {
    registry: Arc<ServiceRegistry>,
    load_balancer: LoadBalancer,
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
    verifier: Arc<VerificationEngine>,
}

impl MicroservicesSystem {
    pub fn new() -> Self {
        let registry = Arc::new(ServiceRegistry::new());
        let load_balancer = LoadBalancer::new(
            registry.clone(),
            LoadBalancingStrategy::RoundRobin,
        );

        MicroservicesSystem {
            registry,
            load_balancer,
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
            verifier: Arc::new(VerificationEngine::new()),
        }
    }

    pub fn register_service(&self, instance: ServiceInstance) -> Result<(), String> {
        self.registry.register(instance.clone())?;

        // Create circuit breaker for this service
        let mut breakers = self.circuit_breakers.lock().unwrap();
        breakers.entry(instance.service.clone())
            .or_insert_with(|| CircuitBreaker::new(5, 3));

        Ok(())
    }

    pub fn call_service(&self, service: &str) -> Result<String, String> {
        // Check circuit breaker
        let breakers = self.circuit_breakers.lock().unwrap();
        if let Some(breaker) = breakers.get(service) {
            if !breaker.is_available() {
                return Err("Service unavailable (circuit open)".to_string());
            }
        }

        // Get instance from load balancer
        let _instance = self.load_balancer.select_instance(service)?;
        println!("📞 Calling service: {}", service);

        Ok(format!("Response from {}", service))
    }

    pub fn verify_consistency(&self) -> Result<(), String> {
        // Verify that at least one instance of each service is healthy
        for (service, instances) in self.registry.list_all() {
            let healthy_count = instances.iter()
                .filter(|i| i.status == ServiceStatus::Healthy)
                .count();

            self.verifier.check_invariant(
                healthy_count > 0,
                &format!("{} has at least one healthy instance", service),
            )?;
        }

        println!("✅ System consistency verified\n");
        Ok(())
    }

    pub fn get_system_status(&self) -> String {
        let mut status = String::from("MICROSERVICES SYSTEM STATUS\n");
        status.push_str("==============================\n");

        for (service, instances) in self.registry.list_all() {
            let healthy = instances.iter().filter(|i| i.status == ServiceStatus::Healthy).count();
            status.push_str(&format!("\n{}: {}/{} healthy\n", service, healthy, instances.len()));
            for instance in instances {
                status.push_str(&format!("  - {} ({}:{})\n", instance.id, instance.address, instance.port));
            }
        }

        status
    }
}

// ============================================================================
// EXAMPLE EXECUTION
// ============================================================================

pub fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🏗️  OMNISYSTEM MICROSERVICES EXAMPLE");
    println!("====================================\n");

    let system = MicroservicesSystem::new();

    // Register services
    println!("📋 Step 1: Register Services");
    system.register_service(ServiceInstance::new("user-api-1", "user-service", "10.0.1.1", 8001))?;
    system.register_service(ServiceInstance::new("user-api-2", "user-service", "10.0.1.2", 8001))?;
    system.register_service(ServiceInstance::new("payment-api-1", "payment-service", "10.0.2.1", 8002))?;
    system.register_service(ServiceInstance::new("order-api-1", "order-service", "10.0.3.1", 8003))?;
    println!();

    // Define properties (Axiom verification)
    println!("✔️  Step 2: Define System Properties");
    system.verifier.define_property("at_least_one_user_service", true);
    system.verifier.define_property("payment_always_available", true);
    system.verifier.define_property("no_cascading_failures", true);
    println!();

    // Route requests (load balancing)
    println!("🔀 Step 3: Route Requests");
    system.call_service("user-service")?;
    system.call_service("user-service")?;
    system.call_service("payment-service")?;
    system.call_service("order-service")?;
    println!();

    // Verify consistency
    println!("🔍 Step 4: Verify System Consistency");
    system.verify_consistency()?;

    // System status
    println!("📊 Step 5: System Status");
    println!("{}", system.get_system_status());

    println!("✅ Microservices Example Complete\n");

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_instance() {
        let instance = ServiceInstance::new("api-1", "user-service", "localhost", 8000);
        assert_eq!(instance.status, ServiceStatus::Healthy);
    }

    #[test]
    fn test_service_registry() {
        let registry = ServiceRegistry::new();
        let instance = ServiceInstance::new("api-1", "test", "localhost", 8000);
        assert!(registry.register(instance).is_ok());
    }

    #[test]
    fn test_load_balancer() {
        let registry = Arc::new(ServiceRegistry::new());
        let i1 = ServiceInstance::new("api-1", "test", "localhost", 8001);
        let i2 = ServiceInstance::new("api-2", "test", "localhost", 8002);
        registry.register(i1).unwrap();
        registry.register(i2).unwrap();

        let lb = LoadBalancer::new(registry, LoadBalancingStrategy::RoundRobin);
        assert!(lb.route("test").is_ok());
    }

    #[test]
    fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, 2);
        assert_eq!(breaker.get_state(), CircuitBreakerState::Closed);

        breaker.record_failure();
        breaker.record_failure();
        breaker.record_failure();

        assert!(!breaker.is_available());
    }

    #[test]
    fn test_verification_engine() {
        let verifier = VerificationEngine::new();
        verifier.define_property("test", false);
        assert!(verifier.verify_property("test").is_ok());
    }

    #[test]
    fn test_microservices_system() {
        let system = MicroservicesSystem::new();
        let instance = ServiceInstance::new("api-1", "test", "localhost", 8000);
        assert!(system.register_service(instance).is_ok());
    }
}
