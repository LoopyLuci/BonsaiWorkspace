# API Reference - Quick Lookup
## Function Reference for All 7 Languages
**Total: 15,700+ Functions | Quick Search Guide**

---

## Finding Functions

### By Language

| Language | Functions | Quick Links |
|----------|-----------|-------------|
| **TITAN** | 3,000+ | [String](#titan-strings) • [Math](#titan-math) • [File](#titan-files) • [Net](#titan-net) • [Crypto](#titan-crypto) |
| **SYLVA** | 1,500+ | [Tensors](#sylva-tensors) • [NN](#sylva-nn) • [ML](#sylva-ml) • [NLP](#sylva-nlp) • [Vision](#sylva-vision) |
| **AETHER** | 1,200+ | [Services](#aether-services) • [Consensus](#aether-consensus) • [Messages](#aether-messages) |
| **AXIOM** | 800+ | [Proofs](#axiom-proofs) • [Verify](#axiom-verify) |
| **HELIX** | 1,500+ | [Graphics](#helix-graphics) • [Physics](#helix-physics) • [Game](#helix-game) |
| **VERA** | 1,200+ | [Components](#vera-components) • [State](#vera-state) • [HTTP](#vera-http) |
| **NEXUS** | 1,000+ | [UI](#nexus-ui) • [Sensors](#nexus-sensors) • [Hardware](#nexus-hardware) |

---

## TITAN - Systems Programming (3,000+)

### String Functions (80+)
```titan
strlen(s: String) -> i32
substr(s: String, start: i32, len: i32) -> String
concat(s1: String, s2: String) -> String
trim(s: String) -> String
split(s: String, delimiter: String) -> Vec<String>
replace(s: String, from: String, to: String) -> String
uppercase(s: String) -> String
lowercase(s: String) -> String
contains(s: String, substring: String) -> bool
```

### Math Functions (165+)
```titan
abs(n: i64) -> i64
sqrt(n: f64) -> f64
pow(base: f64, exp: f64) -> f64
sin(x: f64) -> f64
cos(x: f64) -> f64
tan(x: f64) -> f64
log(x: f64) -> f64
exp(x: f64) -> f64
floor(x: f64) -> i64
ceil(x: f64) -> i64
```

### Cryptography (160+)
```titan
sha256(data: Vec<u8>) -> Vec<u8>
sha512(data: Vec<u8>) -> Vec<u8>
blake2b(data: Vec<u8>) -> Vec<u8>
aes_encrypt(data: Vec<u8>, key: AesKey) -> Vec<u8>
aes_decrypt(data: Vec<u8>, key: AesKey) -> Vec<u8>
generate_random(size: i32) -> Vec<u8>
hmac_sha256(data: Vec<u8>, key: Vec<u8>) -> Vec<u8>
```

### File I/O (150+)
```titan
File::open(path: String, mode: String) -> Result<File>
file.read() -> Vec<u8>
file.write(data: Vec<u8>) -> i32
file.close() -> i32
File::exists(path: String) -> bool
File::delete(path: String) -> i32
File::size(path: String) -> i64
Directory::create(path: String) -> i32
Directory::list(path: String) -> Vec<String>
```

### Networking (250+)
```titan
TcpClient::connect(host: String, port: i32) -> TcpClient
client.send(data: Vec<u8>) -> i32
client.receive() -> Vec<u8>
client.disconnect() -> i32
http_get(url: String) -> HttpResponse
http_post(url: String, body: Vec<u8>) -> HttpResponse
WebSocket::connect(url: String) -> WebSocket
dns_resolve(hostname: String) -> String
```

### Concurrency (200+)
```titan
spawn_thread(closure: fn()) -> i32
spawn_async(closure: async fn()) -> Task
thread_join(thread_id: i32) -> i32
Mutex::new<T>(data: T) -> Mutex<T>
mutex.lock() -> T
RwLock::new<T>(data: T) -> RwLock<T>
Channel::new<T>() -> (Sender<T>, Receiver<T>)
channel.send(data: T) -> i32
channel.recv() -> Option<T>
```

---

## SYLVA - Data Science & AI (1,500+)

### Tensor Operations (200+)
```sylva
zeros(shape: Vec<i32>) -> Tensor
ones(shape: Vec<i32>) -> Tensor
randn(shape: Vec<i32>) -> Tensor
range(start: f64, end: f64, step: f64) -> Tensor
tensor.reshape(new_shape: Vec<i32>) -> Tensor
tensor.transpose() -> Tensor
tensor.flatten() -> Tensor
tensor_add(a: Tensor, b: Tensor) -> Tensor
tensor_matmul(a: Tensor, b: Tensor) -> Tensor
tensor.mean() -> f64
tensor.sum() -> f64
```

### Neural Networks (400+)
```sylva
create_dense_layer(in_features: i32, out_features: i32) -> DenseLayer
create_conv2d(in_channels: i32, out_channels: i32, kernel_size: i32) -> ConvLayer
create_lstm(input_size: i32, hidden_size: i32, num_layers: i32) -> RecurrentLayer
create_transformer(d_model: i32, num_heads: i32) -> TransformerLayer
forward_dense(layer: DenseLayer, input: Tensor) -> Tensor
forward_conv2d(layer: ConvLayer, input: Tensor) -> Tensor
relu(tensor: Tensor) -> Tensor
sigmoid(tensor: Tensor) -> Tensor
softmax(tensor: Tensor, dim: i32) -> Tensor
```

### Training (100+)
```sylva
Adam(learning_rate: f64) -> Optimizer
SGD(learning_rate: f64, momentum: f64) -> Optimizer
CrossEntropyLoss() -> LossFunction
MSELoss() -> LossFunction
backward(loss: Tensor, engine: TensorEngine) -> ()
optimizer.step() -> ()
optimizer.zero_grad() -> ()
clip_gradients(gradients: Vec<Tensor>, max_norm: f64) -> Vec<Tensor>
```

### NLP (300+)
```sylva
tokenize_bpe(text: String) -> Vec<i32>
tokenize_wordpiece(text: String) -> Vec<i32>
word2vec(corpus: Vec<String>, dims: i32) -> Tensor
glove(corpus: Vec<String>, dims: i32) -> Tensor
fasttext(corpus: Vec<String>, dims: i32) -> Tensor
named_entity_recognition(text: String) -> Vec<(String, String)>
analyze_sentiment(text: String) -> (String, f64)
question_answering(context: String, question: String) -> String
machine_translation(text: String, source_lang: String, target_lang: String) -> String
```

### Computer Vision (200+)
```sylva
image.resize(height: i32, width: i32) -> Tensor
image.normalize(mean: Vec<f64>, std: Vec<f64>) -> Tensor
yolo_detect(image: Tensor) -> Vec<Detection>
semantic_segment(image: Tensor) -> Tensor
instance_segment(image: Tensor) -> Vec<Tensor>
face_detection(image: Tensor) -> Vec<BBox>
pose_estimation(image: Tensor) -> Vec<Keypoint>
optical_flow(frame1: Tensor, frame2: Tensor) -> Tensor
```

### Reinforcement Learning (150+)
```sylva
DQNAgent::new(state_size: i32, action_size: i32) -> DQNAgent
agent.choose_action(state: Tensor) -> i32
agent.remember(experience: Experience) -> ()
agent.train(batch_size: i32) -> ()
experience_replay(memory: Vec<Experience>, batch_size: i32) -> Vec<Experience>
epsilon_greedy(q_values: Vec<f64>, epsilon: f64) -> i32
```

---

## AETHER - Distributed Systems (1,200+)

### Service Management (250+)
```aether
ServiceRegistry::new() -> ServiceRegistry
registry.register(service: ServiceDescriptor) -> ()
registry.discover(service_name: String) -> Vec<String>
LoadBalancer::new(algorithm: String) -> LoadBalancer
load_balancer.select_node(nodes: Vec<String>) -> String
CircuitBreaker::new() -> CircuitBreaker
circuit_breaker.call(func: fn()) -> Result
```

### Consensus (200+)
```aether
raft_election(nodes: Vec<RaftNode>) -> RaftNode
raft_append_entries(leader: RaftNode) -> ()
pbft_consensus(nodes: Vec<Node>, request: Vec<u8>) -> bool
paxos_prepare(proposer_id: i32, proposal_number: i32) -> ()
byzantine_fault_tolerance(validators: Vec<Validator>) -> bool
```

### Messaging (180+)
```aether
Channel::new<T>() -> (Publisher<T>, Subscriber<T>)
publisher.publish(message: T) -> i32
subscriber.subscribe(topic: String) -> ()
Queue::new() -> Queue
queue.enqueue(message: Message) -> ()
queue.dequeue() -> Message
create_event_stream() -> EventStream
publish_event(stream: EventStream, event: Event) -> ()
```

### Replication (100+)
```aether
leader_follower_replicate(leader: String, followers: Vec<String>) -> bool
multi_leader_replicate(leaders: Vec<String>) -> bool
peer_to_peer_sync(peers: Vec<String>) -> bool
create_crdt() -> CRDT
crdt.merge(other: CRDT) -> CRDT
```

---

## HELIX - Game Development (1,500+)

### Graphics (350+)
```helix
create_deferred_pipeline(width: i32, height: i32) -> RenderingPipeline
create_ray_tracing_pipeline(width: i32, height: i32) -> RenderingPipeline
create_pbr_material(albedo: Vec<f64>, roughness: f64, metallic: f64) -> Material
compile_shader(source: String, shader_type: String) -> Shader
create_texture_2d(width: i32, height: i32, format: String) -> Texture
apply_post_process(pipeline: RenderingPipeline, effect: String) -> RenderingPipeline
```

### Physics (200+)
```helix
create_physics_world(gravity: Vec3) -> PhysicsWorld
create_rigid_body(mass: f64, shape: CollisionShape) -> RigidBody
raycast(world: PhysicsWorld, origin: Vec3, direction: Vec3) -> Option<RaycastHit>
spherecast(world: PhysicsWorld, origin: Vec3, radius: f64) -> Option<RaycastHit>
overlap_sphere(world: PhysicsWorld, center: Vec3, radius: f64) -> Vec<i32>
update_physics(world: PhysicsWorld, delta_time: f64) -> PhysicsWorld
```

### Game Systems (150+)
```helix
create_entity(scene: Scene, name: String) -> Entity
add_component(entity: Entity, component: Component) -> Entity
get_component(entity: Entity, component_type: String) -> Option<Component>
find_entities(scene: Scene, query: ComponentQuery) -> Vec<Entity>
execute_systems(scene: Scene) -> Scene
```

### Animation (100+)
```helix
create_animation_clip(name: String, duration: f64) -> AnimationClip
sample_animation(clip: AnimationClip, time: f64) -> Vec<f64>
blend_animations(clip1: AnimationClip, clip2: AnimationClip, blend: f64) -> Vec<f64>
create_state_machine(states: Vec<String>) -> StateMachine
```

---

## VERA - Web Development (1,200+)

### Components (300+)
```vera
create_element(tag: String, props: Map, children: Vec) -> VNode
create_fragment(children: Vec<VNode>) -> Fragment
use_state<T>(initial: T) -> (T, fn(T))
use_effect(effect_fn: fn(), dependencies: Vec<String>) -> ()
use_reducer<T>(reducer: fn, initial: T) -> (T, fn)
use_context<T>(context: Context<T>) -> T
use_memo<T>(compute: fn() -> T, deps: Vec) -> T
```

### State Management (150+)
```vera
create_store<T>(initial: T) -> Store<T>
dispatch(store: Store<T>, action: Action) -> Store<T>
subscribe(store: Store<T>, listener: fn) -> Store<T>
time_travel(store: Store<T>, index: i32) -> Store<T>
devtools_middleware() -> Middleware
```

### Routing (120+)
```vera
create_router(routes: Vec<Route>) -> Router
navigate(router: Router, path: String) -> Router
navigate_with_params(router: Router, path: String, params: RouteParams) -> Router
back(router: Router) -> Router
forward(router: Router) -> Router
```

### HTTP (150+)
```vera
create_http_client(base_url: String) -> HttpClient
http_get(client: HttpClient, path: String) -> HttpResponse
http_post(client: HttpClient, path: String, body: Vec<u8>) -> HttpResponse
http_put(client: HttpClient, path: String, body: Vec<u8>) -> HttpResponse
http_delete(client: HttpClient, path: String) -> HttpResponse
graphql_query(client: GraphQLClient, query: String) -> HttpResponse
```

---

## NEXUS - Mobile & IoT (1,000+)

### UI (150+)
```nexus
create_screen(name: String, layout: String) -> Screen
add_button(screen: Screen, text: String, callback: fn) -> Button
add_text_view(screen: Screen, text: String) -> TextView
add_image(screen: Screen, path: String) -> Image
on_click(component: Component, callback: fn) -> Component
```

### Sensors (100+)
```nexus
start_sensor(sensor_type: String) -> Sensor
get_accelerometer_data() -> Vec<f64>
get_gyroscope_data() -> Vec<f64>
get_gps_location() -> Location
```

### Hardware (100+)
```nexus
gpio_set_output(pin: i32, value: bool) -> i32
gpio_read_input(pin: i32) -> bool
pwm_set(pin: i32, value: f64) -> i32
i2c_write(device: i32, data: Vec<u8>) -> i32
i2c_read(device: i32, size: i32) -> Vec<u8>
spi_transfer(data: Vec<u8>) -> Vec<u8>
```

### Connectivity (100+)
```nexus
wifi_connect(ssid: String, password: String) -> bool
bluetooth_scan() -> Vec<Device>
bluetooth_connect(device: Device) -> BluetoothConnection
nfc_read() -> Vec<u8>
nfc_write(data: Vec<u8>) -> bool
```

---

## AXIOM - Formal Verification (800+)

### Theorem Proving (250+)
```axiom
prove_by_induction(base: Formula, step: Formula) -> Proof
prove_by_cases(cases: Vec<Formula>) -> Proof
rewrite(target: Formula, rule: Formula) -> Formula
simp(formula: Formula) -> Formula
omega(formula: Formula) -> Formula
decide(formula: Formula) -> bool
```

### Model Checking (200+)
```axiom
ltl_model_check(system: TransitionSystem, property: String) -> ModelCheckingResult
ctl_model_check(system: TransitionSystem, property: String) -> ModelCheckingResult
mtl_model_check(system: TransitionSystem, property: String) -> ModelCheckingResult
safety_property_check(system: TransitionSystem, bad_states: Vec) -> bool
liveness_property_check(system: TransitionSystem, target_states: Vec) -> bool
```

### Program Verification (150+)
```axiom
hoare_verify(precond: Formula, program: Program, postcond: Formula) -> bool
weakest_precondition(stmt: Statement, postcond: Formula) -> Formula
strongest_postcondition(stmt: Statement, precond: Formula) -> Formula
verify_invariant(loop: Statement, invariant: Formula) -> bool
verify_termination(loop: Statement, metric: String) -> bool
```

---

## Cross-Language Bridges (140+)

### TITAN ↔ SYLVA (20)
```
sylva::load_csv_as_tensor(path: String) -> Tensor
sylva::load_model(path: String) -> NeuralNetwork
sylva::inference(model, input) -> Tensor
```

### SYLVA ↔ AETHER (15)
```
aether::create_model_service(model, port) -> Service
aether::inference_request(model_id, input) -> Tensor
```

### VERA ↔ SYLVA (10)
```
sylva::load_webassembly_model(path) -> Model
vera::render_chart(data) -> VNode
```

---

## Search Tips

**By functionality:**
- String manipulation → TITAN
- Data processing → SYLVA
- Network communication → AETHER
- Safety proofs → AXIOM
- 3D graphics → HELIX
- Web UI → VERA
- Mobile → NEXUS

**By use case:**
- API server → TITAN + AETHER
- ML model → SYLVA
- Game → HELIX
- Web app → VERA
- Mobile app → NEXUS
- Verified system → AXIOM

---

## Need More Detail?

- **[Language Guides](LANGUAGES.md)** — Complete language documentation
- **[Getting Started](GETTING_STARTED.md)** — Tutorial walkthrough
- **[Code Examples](EXAMPLES.md)** — Real working code
- **[Advanced Features](ADVANCED_FEATURES.md)** — Quantum, blockchain, AI

---

🔍 **Can't find a function? [Check FAQ](FAQ.md) or [See Examples](EXAMPLES.md)**
