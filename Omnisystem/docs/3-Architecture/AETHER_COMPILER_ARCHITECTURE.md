# AETHER Compiler Architecture v1.0
## Distributed/Async Compilation System

---

## 1. COMPILATION PIPELINE

```
AETHER Source Code
    ↓
[Lexer] → Tokens
    ↓
[Parser] → Actor/Async AST
    ↓
[Type Checker] → Typed AST with Concurrency Constraints
    ↓
[Lifetime Analyzer] → Message Lifetime Verification
    ↓
[Distributed Analysis] → Network Partition Detection
    ↓
[Scheduler Optimizer] → Task Scheduling Plan
    ↓
[Code Generator] → LLVM IR with Runtime Hooks
    ↓
[Runtime Linker] → Executable with AETHER Runtime
    ↓
[Native Binary] → Distributed-Ready Application
```

---

## 2. ACTOR MODEL PARSING

### 2.1 Actor AST Structure

```
Actor
├── name: string
├── state: StateDecl[]
├── messages: MessageHandler[]
├── lifecycle_hooks: LifecycleHook[]
├── remote_config: RemoteConfig?
└── supervision_config: SupervisionConfig?

MessageHandler
├── message_name: string
├── message_type: Type
├── parameters: Parameter[]
├── handler_body: Block
└── reply_type: Type?

StateDecl
├── name: string
├── type: Type
├── initial_value: Expression
└── is_mutable: bool
```

### 2.2 Parser Implementation

```
class ActorParser {
    fn parse_actor() -> Actor {
        expect("actor")
        name = expect("identifier")
        
        state_decls = []
        message_handlers = []
        lifecycle_hooks = []
        
        expect("{")
        
        while not check("}") {
            if match("state") {
                state_decls.push(parse_state())
            } else if match("message") {
                message_handlers.push(parse_message_handler())
            } else if match("on_") {
                lifecycle_hooks.push(parse_lifecycle_hook())
            } else if check("identifier") && peek() == "{" {
                // Async function
            }
        }
        
        expect("}")
        
        return Actor(name, state_decls, message_handlers, lifecycle_hooks)
    }
    
    fn parse_message_handler() -> MessageHandler {
        expect("message")
        message_name = expect("identifier")
        
        expect("(")
        params = parse_parameters()
        expect(")")
        
        return_type = null
        if match("->") {
            return_type = parse_type()
        }
        
        body = parse_block()
        
        return MessageHandler(message_name, params, body, return_type)
    }
    
    fn parse_async_function() -> AsyncFunction {
        expect("async")
        expect("fn")
        name = expect("identifier")
        
        expect("(")
        params = parse_parameters()
        expect(")")
        
        return_type = parse_return_type()
        body = parse_async_block()
        
        return AsyncFunction(name, params, return_type, body)
    }
    
    fn parse_async_block() -> AsyncBlock {
        // Track await points and concurrent operations
        statements = []
        await_points = []
        
        expect("{")
        
        while not check("}") {
            if match("await") {
                expr = parse_expression()
                statements.push(AwaitStatement(expr))
                await_points.push(expr)
            } else if match("spawn") {
                expr = parse_expression()
                statements.push(SpawnStatement(expr))
            } else {
                statements.push(parse_statement())
            }
        }
        
        expect("}")
        
        return AsyncBlock(statements, await_points)
    }
}
```

---

## 3. TYPE CHECKING FOR CONCURRENCY

### 3.1 Concurrency Type Constraints

```
fn type_check_actor(actor: Actor) -> Result<TypedActor, Error> {
    env = new Environment()
    
    // Add state variables to environment
    for state in actor.state {
        env.add_binding(state.name, state.type)
    }
    
    // Type check message handlers
    for handler in actor.message_handlers {
        type_check_message_handler(handler, env)
    }
    
    // Check serializable for distributed actors
    if actor.is_distributed() {
        for state in actor.state {
            if not is_serializable(state.type) {
                error("Actor state must be serializable for distributed use")
            }
        }
        
        for handler in actor.message_handlers {
            if not is_serializable(handler.message_type) {
                error("Messages must be serializable for distributed actors")
            }
        }
    }
    
    return Ok(TypedActor(actor, env))
}

fn type_check_async_function(fn_decl: AsyncFunction) -> Result<TypedAsyncFn, Error> {
    env = new Environment()
    
    // Check await points are on async values
    for await_point in fn_decl.body.await_points {
        await_type = infer_type(await_point, env)
        
        if not is_future_or_async(await_type) {
            error("Can only await Future or async function, got {}", await_type)
        }
    }
    
    // Check no blocking operations in async context
    for stmt in fn_decl.body.statements {
        if is_blocking_operation(stmt) {
            error("Blocking operation in async context: {}", stmt)
        }
    }
    
    return Ok(TypedAsyncFn(fn_decl, env))
}
```

---

## 4. DISTRIBUTED ANALYSIS

### 4.1 Serialization Verification

```
fn analyze_distributed_actor(actor: TypedActor) -> Result<DistributedActorInfo, Error> {
    // Verify all messages can be serialized
    for handler in actor.message_handlers {
        if not can_serialize(handler.message_type) {
            error("Cannot serialize message type: {}", handler.message_type)
        }
        
        if handler.reply_type && not can_serialize(handler.reply_type) {
            error("Cannot serialize reply type: {}", handler.reply_type)
        }
    }
    
    // Verify state is serializable for persistence
    for state in actor.state {
        if not can_serialize(state.type) {
            error("Cannot serialize state: {}", state.name)
        }
    }
    
    // Generate serialization code
    serialization_code = generate_serialization_code(actor)
    
    return Ok(DistributedActorInfo(actor, serialization_code))
}

fn analyze_network_partition_tolerance(system: TypedProgram) -> PartitionAnalysis {
    analysis = PartitionAnalysis {}
    
    // Track message dependencies
    for actor in system.actors {
        for other_actor in system.actors {
            messages = get_messages_from(actor, other_actor)
            if messages.is_not_empty() {
                analysis.add_dependency(actor, other_actor)
            }
        }
    }
    
    // Check for circular dependencies (deadlock potential)
    cycles = find_cycles(analysis.dependency_graph)
    if cycles.is_not_empty() {
        analysis.warning("Circular actor dependencies detected - potential deadlock")
    }
    
    // Check consistency guarantees
    for actor in system.actors {
        if actor.is_distributed() && !has_eventual_consistency_handling(actor) {
            analysis.warning("Distributed actor {} may have consistency issues", actor.name)
        }
    }
    
    return analysis
}
```

---

## 5. ASYNC/AWAIT TRANSFORMATION

### 5.1 Async State Machine Transformation

```
fn transform_async_function(async_fn: AsyncFunction) -> StateMachine {
    // Convert async/await into state machine
    // Each await point becomes a state transition
    
    states = []
    current_state = 0
    
    for (i, stmt) in async_fn.body.statements.enumerate() {
        if stmt is AwaitStatement {
            // Create state for await point
            states.push(State {
                id: current_state,
                code_before_await: stmts[0..i],
                await_expression: stmt.expression,
                continuation_state: current_state + 1
            })
            current_state += 1
        } else if stmt is SpawnStatement {
            // Create new task
            task_id = spawn_new_task(stmt.expression)
            states.push(State {
                id: current_state,
                spawn_expression: stmt.expression,
                task_id: task_id
            })
            current_state += 1
        }
    }
    
    // Final state
    states.push(State {
        id: current_state,
        code: async_fn.body.final_statements,
        is_final: true
    })
    
    return StateMachine(states, async_fn.name)
}

fn generate_state_machine_code(sm: StateMachine) -> string {
    code = "struct {}State {{\n".format(sm.name)
    code += "  state: u32,\n"
    
    // Variables that persist across await points
    for var in sm.persistent_variables {
        code += "  {}: {},\n".format(var.name, var.type.llvm_name())
    }
    
    code += "}\n\n"
    
    code += "fn {}(state_machine: &mut {}State) -> Option<{}> {{\n".format(
        sm.name, sm.name, sm.return_type.llvm_name()
    )
    
    code += "  match state_machine.state {\n"
    
    for state in sm.states {
        code += "    {} => {{\n".format(state.id)
        code += generate_state_code(state)
        code += "    }\n"
    }
    
    code += "  }\n"
    code += "}\n"
    
    return code
}
```

---

## 6. SCHEDULER OPTIMIZATION

### 6.1 Task Scheduling

```
fn optimize_scheduling(async_fn: TypedAsyncFn) -> SchedulingPlan {
    plan = SchedulingPlan {}
    
    // Identify independent tasks
    dependencies = analyze_task_dependencies(async_fn)
    
    for (i, task) in async_fn.tasks.enumerate() {
        independent_tasks = find_independent_tasks(task, dependencies)
        
        // Can run concurrently
        plan.parallel_groups.push(independent_tasks)
    }
    
    // Estimate task duration
    for task in async_fn.tasks {
        duration = estimate_duration(task)
        plan.task_durations[task] = duration
    }
    
    // Reorder for cache locality
    plan.reorder_for_locality(async_fn)
    
    return plan
}

fn analyze_actor_scheduling(system: TypedProgram) -> ActorSchedulingPlan {
    plan = ActorSchedulingPlan {}
    
    // Group actors by affinity
    for actor in system.actors {
        affinity_group = determine_affinity_group(actor)
        plan.affinity_groups[affinity_group].push(actor)
    }
    
    // Schedule message processing order
    for actor in system.actors {
        message_priority = determine_message_priority(actor)
        plan.message_priorities[actor] = message_priority
    }
    
    // Load balance across cores
    core_assignments = load_balance(plan.affinity_groups, system.available_cores)
    plan.core_assignments = core_assignments
    
    return plan
}
```

---

## 7. CODE GENERATION

### 7.1 Actor Code Generation

```
fn generate_actor_code(actor: TypedActor) -> LLVMModule {
    module = LLVMModule::new()
    
    // Actor state structure
    state_struct = module.create_struct(actor.name + "State")
    for state_var in actor.state {
        state_struct.add_field(state_var.name, state_var.type.llvm_type())
    }
    
    // Message queue type
    message_queue_type = module.create_message_queue_type(actor)
    
    // Actor instance structure
    actor_struct = module.create_struct(actor.name)
    actor_struct.add_field("state", state_struct)
    actor_struct.add_field("message_queue", message_queue_type)
    actor_struct.add_field("actor_ref", i64)  // Global actor reference
    
    // Generate message handlers
    for handler in actor.message_handlers {
        fn = module.create_function(actor.name + "_" + handler.message_name)
        fn.add_parameter("actor", actor_struct.pointer_type())
        fn.add_parameter("message", handler.message_type.llvm_type())
        
        builder = IRBuilder(fn)
        
        // Generate handler body
        generate_handler_body(handler, builder, actor_struct)
        
        // Generate reply if applicable
        if handler.reply_type {
            generate_reply_code(handler, builder)
        }
    }
    
    // Generate process_message dispatcher
    dispatcher = module.create_function(actor.name + "_process_message")
    dispatcher.add_parameter("actor", actor_struct.pointer_type())
    dispatcher.add_parameter("message", message_queue_type.element_type())
    
    builder = IRBuilder(dispatcher)
    
    // Switch on message type
    for handler in actor.message_handlers {
        case = builder.create_case(handler.message_name)
        call_handler = builder.create_call(
            actor.name + "_" + handler.message_name,
            [actor, message]
        )
    }
    
    return module
}

fn generate_async_code(async_fn: TypedAsyncFn, scheduling_plan: SchedulingPlan) -> LLVMModule {
    module = LLVMModule::new()
    
    // Create future type
    future_type = module.create_struct("Future_" + async_fn.name)
    future_type.add_field("state", i32)
    future_type.add_field("result", async_fn.return_type.llvm_type())
    future_type.add_field("waker", i64)  // Waker for resumption
    
    // Variables persisting across await
    for var in async_fn.persistent_variables {
        future_type.add_field(var.name, var.type.llvm_type())
    }
    
    // Generate state machine
    sm_code = transform_async_function(async_fn)
    module.add_code(generate_state_machine_code(sm_code))
    
    // Generate poll function (executor calls this)
    poll_fn = module.create_function(async_fn.name + "_poll")
    poll_fn.add_parameter("future", future_type.pointer_type())
    
    builder = IRBuilder(poll_fn)
    
    // Call state machine
    state_result = builder.create_call(async_fn.name, [future])
    
    // Return Poll::Ready or Poll::Pending
    builder.create_cond_branch(
        state_result.is_some(),
        ready_block,
        pending_block
    )
    
    return module
}
```

---

## 8. RUNTIME INTEGRATION

### 8.1 AETHER Runtime

```
class AETHERRuntime {
    fn initialize() -> void {
        // Initialize actor system
        actor_registry = ActorRegistry::new()
        message_queue_global = MessageQueueGlobal::new()
        scheduler = Scheduler::new(num_worker_threads)
        
        // Initialize async executor
        executor = AsyncExecutor::new()
        
        // Start worker threads
        for i in 0..num_worker_threads {
            spawn_worker_thread(i)
        }
    }
    
    fn spawn_actor(actor_type: Type, config: ActorConfig) -> ActorRef {
        // Create actor instance
        actor_instance = create_actor_instance(actor_type)
        
        // Register in system
        actor_id = actor_registry.register(actor_instance)
        
        // If remote, setup network endpoint
        if config.remote_node {
            setup_remote_endpoint(actor_id, config.remote_node)
        }
        
        return ActorRef(actor_id)
    }
    
    fn send_message(actor_ref: ActorRef, message: Message) -> void {
        // Queue message for actor
        message_queue_global.enqueue(actor_ref.id, message)
        
        // Wake up executor
        scheduler.notify_new_work()
    }
    
    fn await_result(future: Future<T>) -> T {
        // Block until future is ready
        while !future.is_ready() {
            executor.poll(future)
        }
        
        return future.result
    }
}

class Scheduler {
    fn run_event_loop() -> void {
        loop {
            // Process actor messages
            while let Some((actor_id, message)) = message_queue_global.dequeue() {
                actor = actor_registry.get(actor_id)
                process_message(actor, message)
            }
            
            // Execute async tasks
            executor.run_until_stalled()
            
            // Check for work
            if !has_pending_work() {
                block_on_io()
            }
        }
    }
}
```

---

## 9. EXAMPLE: COMPLETE COMPILATION

```
AETHER Actor:
──────────────
actor Counter {
    state count: i32 = 0
    
    message Increment {
        count = count + 1
        reply(count)
    }
}

Async Function:
────────────────
async fn increment_multiple(counter: ActorRef, times: i32) -> i32 {
    for i in 0..times {
        let new_count = await counter.call(Counter::Increment)
        println("Count: {}", new_count)
    }
    return counter.call(Counter::Increment)
}

After Parsing:
───────────────
Actor {
    name: "Counter",
    state: [StateDecl { name: "count", type: i32, init: 0 }],
    messages: [MessageHandler { 
        name: "Increment",
        body: Assignment(count, Add(count, 1))
    }]
}

After Type Checking:
─────────────────────
TypedActor(Counter) ✓
TypedAsyncFn(increment_multiple) with 5 await points

After Async Transformation:
────────────────────────────
StateMachine {
    states: [
        State 0: load counter ref
        State 1: await increment (point 1)
        State 2: await increment (point 2)
        ...
        State 5: final return
    ]
}

Generated LLVM IR:
───────────────────
%Counter.State = type { i32 }

define i32 @Counter_process_message(%Counter* %actor, i32 %message_type) {
    switch i32 %message_type, label %default [
        i32 0, label %handle_increment
    ]
    
    handle_increment:
        %state_ptr = getelementptr %Counter, %Counter* %actor, i32 0, i32 0
        %count = load i32, i32* %state_ptr
        %new_count = add i32 %count, 1
        store i32 %new_count, i32* %state_ptr
        ret i32 %new_count
}

Result: Native executable with integrated AETHER runtime
```

---

This architecture enables AETHER to seamlessly handle distributed systems, async/await patterns, and actor-based concurrency.
