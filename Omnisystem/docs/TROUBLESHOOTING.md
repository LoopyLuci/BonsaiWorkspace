# Troubleshooting Guide

**Solutions to common Omnisystem issues**

---

## Startup Issues

### "Module not found"
**Cause**: Module manifest not loaded  
**Solution**:
```bash
omnisystem module load base-modules/MODULE_MANIFEST.omni
omnisystem module list  # Verify
```

### "Language server failed to start"
**Cause**: LSP port in use or permission error  
**Solution**:
```bash
# Find process on port
lsof -i :8080

# Kill and restart
omnisystem lsp --restart
```

### "Out of memory during compilation"
**Cause**: Large program or memory leak  
**Solution**:
```bash
# Increase heap
omnisystem compile --heap 4g program.ti

# Or split program
# Compile modules separately
```

---

## Runtime Issues

### "Panic: index out of bounds"
**Cause**: Array/vector access out of range  
**Solution**:
```titan
// Check bounds before access
if idx < array.len() {
    let item = array[idx]
}

// Or use safe methods
if let Some(item) = array.get(idx) {
    // Use item safely
}
```

### "Type mismatch error"
**Cause**: Incompatible types being used  
**Solution**:
```titan
// Check type annotations
let x: i32 = 5.0  // ERROR: f64 to i32

// Convert explicitly
let x: i32 = 5.0 as i32  // OK
```

### "Stack overflow"
**Cause**: Infinite recursion or deep stack usage  
**Solution**:
```titan
// Add base case to recursion
fun factorial(n: i32) -> i32 {
    if n <= 1 { return 1 }  // Base case
    n * factorial(n - 1)
}

// Or use iteration
let mut result = 1
for i in 2..=n { result *= i }
```

---

## TITAN Issues

### "Borrow checker error"
**Cause**: Conflicting borrows or moved values  
**Solution**:
```titan
// Use references instead of moving
fn print_vec(v: &Vec<i32>) {  // Borrow
    println!("{:?}", v)
}

let v = vec![1, 2, 3]
print_vec(&v)
print_vec(&v)  // OK
```

### "Unsafe code issues"
**Cause**: Unsafe block with undefined behavior  
**Solution**:
```titan
// Document safety requirements
// Avoid unsafe when possible
// Use safe alternatives:
// - Arc<T> instead of *mut T
// - Box<T> instead of malloc/free
// - Vec<T> instead of raw arrays
```

---

## SYLVA Issues

### "Tensor shape mismatch"
**Cause**: Operations on incompatible tensor shapes  
**Solution**:
```sylva
// Check shapes before operation
if t1.shape() == t2.shape() {
    let result = &t1 + &t2
}

// Or reshape
let t2_reshaped = t2.reshape(t1.shape())?
let result = &t1 + &t2_reshaped
```

### "NaN or Infinity in training"
**Cause**: Numerical instability  
**Solution**:
```sylva
// Clip gradients
optimizer.with_max_grad_norm(1.0)

// Reduce learning rate
let opt = Adam::new(0.0001)  // Lower LR

// Normalize inputs
let normalized = normalize(&input)
```

### "Out of GPU memory"
**Cause**: Batch size too large  
**Solution**:
```sylva
// Reduce batch size
let loader = DataLoader::new(data)
    .with_batch_size(16)  // Was 128

// Or reduce model size
// Or use gradient accumulation
```

---

## AETHER Issues

### "No leader elected"
**Cause**: Network issues or too few nodes  
**Solution**:
```bash
# Check cluster status
omnisystem cluster status

# Check network connectivity
omnisystem cluster ping --all

# Increase election timeout if slow network
# Or ensure at least 3 nodes running
```

### "Replication lag increasing"
**Cause**: Slow network or overloaded followers  
**Solution**:
```bash
# Check network
omnisystem metrics | grep latency

# Reduce write load
omnisystem cluster balance

# Check replica resources
omnisystem cluster status --detailed
```

### "Split brain condition"
**Cause**: Network partition  
**Solution**:
```bash
# Omnisystem prevents this with consensus
# But if it happens:

# 1. Identify partitions
omnisystem cluster diagnose --network

# 2. Restore network
# Fix network connectivity

# 3. Cluster auto-heals
# Leader re-elected, data reconciled
```

---

## AXIOM Issues

### "Proof timeout"
**Cause**: Search space too large  
**Solution**:
```axiom
// Reduce search depth
let prover = TheoremProver::new()
    .with_depth_limit(50)  // Was unlimited

// Or split proof
// Prove lemmas first, then main theorem
```

### "Type inference failed"
**Cause**: Ambiguous types  
**Solution**:
```axiom
// Add explicit type annotations
let f: Formula = Formula::Atom("P")

// Or use concrete types
let t: Type = Type::Int
```

---

## Network Issues

### "Connection timeout"
**Cause**: Network unreachable or port closed  
**Solution**:
```bash
# Check network
ping target-host

# Check port
telnet target-host 5001

# Check firewall
sudo ufw allow 5001/tcp

# Check DNS
nslookup target-host
```

### "TLS handshake failed"
**Cause**: Certificate issues  
**Solution**:
```bash
# Verify certificate
openssl x509 -in cert.pem -text -noout

# Check cert validity
openssl x509 -in cert.pem -noout -dates

# Regenerate if needed
omnisystem cert generate --host localhost
```

---

## Performance Issues

### "High CPU usage"
**Cause**: Inefficient algorithm or tight loop  
**Solution**:
```bash
# Profile
omnisystem profile --cpu

# Identify bottleneck
# Optimize algorithm or use different data structure
```

### "Memory leak"
**Cause**: Unreleased resources  
**Solution**:
```bash
# Profile memory
omnisystem profile --memory

# Check for:
omnisystem logs | grep "allocation"

# Fix:
# - Close files/sockets when done
# - Drop references when no longer needed
# - Use RAII (destructors)
```

### "Slow startup"
**Cause**: Initialization overhead  
**Solution**:
```bash
# Profile startup
omnisystem profile --startup-time

# Optimizations:
# - Lazy initialization
# - Async startup
# - Caching
```

---

## Debugging Techniques

### Enable Debug Logging
```bash
omnisystem run --log-level debug > debug.log
```

### Use Debugger
```bash
omnisystem debug program.ti
# (step, breakpoint, inspect variables)
```

### Print Debugging
```titan
println!("DEBUG: x = {:?}", x)
dbg!(x)  // Prints with context
```

### Collect Diagnostics
```bash
omnisystem diagnostics > diagnostics.txt
# Includes: system info, logs, config, metrics
```

---

## Getting Help

1. Check this document
2. Check relevant language guide
3. Check [FAQ.md](FAQ.md)
4. Check community forum: omnisystem.io/forum
5. Open GitHub issue with diagnostics

---

## Next Steps

- FAQ: [FAQ.md](FAQ.md)
- Operations: [OPERATIONS.md](OPERATIONS.md)
- Performance: [PERFORMANCE.md](PERFORMANCE.md)

---

**Troubleshooting** - Solve issues fast!
