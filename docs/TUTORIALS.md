# Omnisystem Tutorial Guides

## Table of Contents
1. [Getting Started with TITAN](#titan-tutorial)
2. [Building ML Pipelines with SYLVA](#sylva-tutorial)
3. [Distributed Systems with AETHER](#aether-tutorial)
4. [Formal Verification with AXIOM](#axiom-tutorial)

---

## TITAN Tutorial: Systems & Computation Programming

### Chapter 1: Basic Operations

#### Working with Strings
```
// Create and manipulate strings
let greeting = "Hello, World";
let length = string_length(greeting);  // 13

// Concatenation
let message = string_concat("Welcome to ", "Omnisystem");

// Searching
let found = string_contains(message, "Omnisystem");  // 1 (true)

// Case conversion
let upper = string_uppercase("hello");   // "HELLO"
let lower = string_lowercase("HELLO");   // "hello"

// Trimming whitespace
let trimmed = string_trim("  spaces  ");  // "spaces"
```

#### Working with JSON
```
// Creating JSON objects
let user = json_create_object();
let user = json_object_set(user, "name", "Alice");
let user = json_object_set(user, "age", "30");

// Serialization
let json_str = json_stringify(user);  // {"name":"Alice","age":30}

// Pretty printing
let pretty = json_stringify_pretty(user, 2);
// {
//   "name": "Alice",
//   "age": 30
// }

// Parsing JSON
let parsed = json_parse(json_str);
let name = json_object_get(parsed, "name");  // "Alice"
```

#### Mathematical Operations
```
// Basic math
let result = math_sqrt(16.0);   // 4.0
let power = math_power(2.0, 8.0);  // 256.0
let minimum = math_min(10, 5);  // 5

// Trigonometry
let sine = math_sin(1.57);  // ~1.0 (sine of pi/2)
let cosine = math_cos(0.0);  // 1.0

// Rounding
let rounded = math_round(3.7);  // 4
let floored = math_floor(3.7);  // 3
let ceiled = math_ceil(3.2);    // 4
```

### Chapter 2: Error Handling

#### Using Result Type
```
// Function that might fail
fn divide(a: i64, b: i64) -> String {
    if b == 0 then
        result_error("division by zero")
    else
        result_ok(a / b)
    end
}

// Using result
let result = divide(10, 2);

if result_is_ok(result) then
    let value = result_unwrap(result);  // 5
    // ... use value
else
    let error = result_unwrap_err(result);
    // ... handle error
end

// Unwrap with default
let value = result_unwrap_or(result, 0);  // Use 0 if error
```

#### Using Option Type
```
// Function that might return nothing
fn find_user(id: i64) -> String {
    if id > 0 then
        option_some(id)
    else
        option_none()
    end
}

// Using option
let user = find_user(42);

if option_is_some(user) then
    let user_id = option_unwrap(user);
    // ... use user
else
    // ... handle missing user
end

// Unwrap with default
let user_id = option_unwrap_or(user, -1);  // -1 if none
```

### Chapter 3: File Operations

#### Reading & Writing Files
```
// Writing file
let content = "Hello\nWorld\nOmnisystem";
file_write("output.txt", content);

// Reading entire file
let text = file_read_all("output.txt");

// Reading as lines
let lines = file_read_lines("output.txt");
// ["Hello", "World", "Omnisystem"]

// Appending to file
file_append("output.txt", "\n\nAppended text");

// Checking existence
if file_exists("output.txt") then
    let size = file_size("output.txt");
else
    // File not found
end
```

#### Directory Operations
```
// Creating directories
dir_create("output");
dir_create_all("data/processed/results");

// Listing contents
let files = dir_list("output");

// Checking directory
if dir_exists("data") then
    // ... work with directory
end

// Getting current directory
let current = dir_current();

// Creating path
let filepath = path_join("output", "results.txt");
let dirname = path_dirname(filepath);     // "output"
let basename = path_basename(filepath);   // "results.txt"
```

#### Compression
```
// Compressing file content
let data = "Large data content...";
let compressed = gzip_compress(data);

// Decompressing
let decompressed = gzip_decompress(compressed);

// Creating zip archive
zip_compress("files_to_compress");

// Extracting archive
zip_decompress("archive.zip", "output_dir");
```

### Chapter 4: Cryptography

#### Hashing
```
// Creating hashes
let hash = sha256("password");
let md5 = md5("data");

// HMAC signing
let signature = hmac_sha256("message", "secret_key");

// Password hashing
let password_hash = bcrypt_hash("user_password");

// Verifying password
let is_valid = bcrypt_verify("user_input", password_hash);  // 1 if valid
```

#### Encryption
```
// AES encryption
let plaintext = "sensitive data";
let key = "encryption_key_32_chars_required";
let encrypted = aes_encrypt(plaintext, key);

// AES decryption
let decrypted = aes_decrypt(encrypted, key);

// Generating random bytes
let random_data = random_bytes(32);

// Generating UUID
let id = uuid_v4();  // "550e8400-e29b-41d4-a716-446655440000"
```

---

## SYLVA Tutorial: Data Science & Machine Learning

### Chapter 1: Working with DataFrames

#### Creating & Loading Data
```
// Create DataFrame with random data
let df = dataframe_create(100, 5);  // 100 rows, 5 columns

// Load from CSV
let df = dataframe_from_csv("data.csv");

// Load from JSON
let df = dataframe_from_json("data.json");

// Inspect data
let shape = dataframe_shape(df);       // "(100, 5)"
let cols = dataframe_get_column_names(df);
let stats = dataframe_describe(df);    // Statistical summary

// View data
let first_5 = dataframe_head(df, 5);
let last_5 = dataframe_tail(df, 5);
```

#### Data Selection & Filtering
```
// Select columns
let selected = dataframe_select_columns(df, "name,age,salary");

// Drop columns
let reduced = dataframe_drop_columns(df, "unused_col");

// Filter rows
let young = dataframe_filter(df, "age < 30");
let high_earners = dataframe_filter(df, "salary > 100000");

// Multiple conditions
let target = dataframe_filter(df, "age > 25 AND salary > 50000");

// Get specific cell
let value = dataframe_get_cell(df, 0, "age");

// Get specific row
let row = dataframe_get_row(df, 0);
```

#### Data Transformation
```
// Sorting
let by_age = dataframe_sort(df, "age", "asc");
let by_salary = dataframe_sort(df, "salary", "desc");

// Remove duplicates
let unique = dataframe_drop_duplicates(df, "email");

// Handle missing values
let cleaned = dataframe_drop_missing(df);
let filled = dataframe_fill_missing(df, "0");

// Normalization
let normalized = normalize_minmax(df);
let standardized = standardize(df);
```

#### Grouping & Aggregation
```
// Group by department
let by_dept = dataframe_groupby(df, "department");

// Calculate aggregates
let salary_by_dept = groupby_sum(by_dept);
let avg_salary = groupby_mean(by_dept);
let dept_size = groupby_count(by_dept);

// Pivoting
let pivot = dataframe_pivot(df, "year", "quarter", "revenue");

// Concatenating DataFrames
let combined = dataframe_concat("df1,df2", 0);  // Stack rows

// Joining DataFrames
let joined = dataframe_join(df1, df2, "employee_id");
let left = dataframe_left_join(df1, df2, "id");
```

#### Statistical Analysis
```
// Column statistics
let mean = mean_column(df, "salary");
let median = median_column(df, "age");
let std = std_column(df, "salary");
let variance = var_column(df, "score");

// Min/max values
let min_age = min_column(df, "age");
let max_age = max_column(df, "age");

// Quantiles
let q75 = quantile_column(df, "salary", 0.75);

// Correlations
let correlation = correlation(df, "age", "salary");
```

### Chapter 2: Machine Learning

#### Training Models
```
// Random Forest (classification/regression)
let features = dataframe_select_columns(df, "feature1,feature2,feature3");
let labels = dataframe_select_columns(df, "target");
let model = random_forest(features, labels, 100);  // 100 trees

// Linear Regression
let regression_model = linear_regression(features, labels);

// Logistic Regression (binary classification)
let classifier = logistic_regression(features, labels);

// Decision Tree
let tree = decision_tree(features, labels);

// Neural Network
let nn = neural_net_create(10, 50, 1);  // Input: 10, Hidden: 50, Output: 1
let trained_nn = neural_net_train(nn, features, labels);

// K-Means Clustering
let clusters = kmeans(features, 5);  // 5 clusters

// Isolation Forest (anomaly detection)
let anomalies = isolation_forest(features, 0.05);  // 5% contamination
```

#### Making Predictions
```
// Predictions with trained model
let predictions = model_predict(model, new_features);

// Probability predictions
let probabilities = model_predict_proba(model, new_features);

// Example: Predicting house prices
let test_data = dataframe_select_columns(test_df, "rooms,location,age");
let prices = model_predict(trained_model, test_data);
```

#### Model Evaluation
```
// Accuracy
let acc = accuracy(predictions, true_labels);

// Precision, Recall, F1
let prec = precision(predictions, true_labels);
let rec = recall(predictions, true_labels);
let f1 = f1_score(predictions, true_labels);

// Confusion Matrix
let cm = confusion_matrix(predictions, true_labels);

// ROC AUC Score
let auc = roc_auc(predictions, true_labels);

// Cross-validation
let cv_scores = cross_validate(model, features, labels, 5);  // 5-fold
```

#### Feature Engineering
```
// Normalization
let normalized = normalize_minmax(features);  // [0, 1] scale
let standardized = standardize(features);     // Zero mean, unit variance

// One-hot encoding (categorical variables)
let encoded = one_hot_encode(df, "category");

// Label encoding
let labels_encoded = label_encode(df, "category");

// Polynomial features
let poly = polynomial_features(features, 2);  // Degree 2

// Feature selection
let best_features = select_k_best(features, labels, 5);

// Principal Component Analysis
let pca_features = pca(features, 3);  // 3 principal components

// Feature importance
let importance = feature_importance(model);
```

### Chapter 3: Natural Language Processing

#### Text Processing
```
// Tokenization
let tokens = tokenize("Hello world from Omnisystem");
// ["Hello", "world", "from", "Omnisystem"]

// Remove stopwords
let filtered = remove_stopwords(tokens, "english");
// ["Hello", "world", "Omnisystem"]

// Stemming
let stemmed = stemming(filtered);
// ["hello", "world", "omnisystem"]

// Lemmatization
let lemmas = lemmatization(filtered);
```

#### Sentiment Analysis
```
// Analyze sentiment
let sentiment = sentiment_analyze("This is amazing! I love it!");
// {"sentiment": "positive", "score": 0.95}

let negative = sentiment_analyze("This is terrible and disappointing.");
// {"sentiment": "negative", "score": 0.92}

// Named entity recognition
let entities = extract_entities("Apple Inc. is located in Cupertino, California.");
// {"entities": [{"text": "Apple Inc.", "type": "ORG"}, ...]}

// Extract keywords
let keywords = extract_keywords(text, 10);  // Top 10 keywords
```

#### Embeddings & Similarity
```
// Generate text embedding
let embedding1 = generate_embedding("machine learning");
let embedding2 = generate_embedding("deep learning");

// Text similarity
let similarity = text_similarity("cat", "dog");  // 0.75

// Semantic search
let query = "best machine learning algorithms";
let documents = load_documents("documents.txt");
let results = semantic_search(query, documents);
// Returns most similar documents
```

### Chapter 4: Time Series Forecasting

#### Time Series Operations
```
// Create time series
let values = "[100, 105, 103, 108, 110]";
let timestamps = "[\"2024-01-01\", \"2024-01-02\", ...]";
let ts = timeseries_create(values, timestamps);

// Resampling
let daily = timeseries_resample(ts, "D");     // Daily
let weekly = timeseries_resample(ts, "W");    // Weekly
let monthly = timeseries_resample(ts, "M");   // Monthly

// Rolling averages
let moving_avg = timeseries_rolling_mean(ts, 7);  // 7-day moving average

// Differencing
let diff = timeseries_difference(ts, 1);  // First order difference
```

#### Forecasting
```
// Simple exponential smoothing
let forecast = timeseries_exponential_smoothing(ts, 0.3);

// ARIMA forecasting
let arima_model = timeseries_arima(ts, "1,1,1");  // ARIMA(1,1,1)
let predictions = timeseries_forecast(arima_model, 30);  // Next 30 periods

// Forecast with confidence intervals
let forecast_with_ci = timeseries_forecast_interval(ts, 30, 0.95);  // 95% CI

// Seasonal decomposition
let decomposed = timeseries_seasonal_decompose(ts, 12);  // 12-month period
// Returns: {"trend": [...], "seasonal": [...], "residual": [...]}
```

---

## AETHER Tutorial: Distributed Systems

### Chapter 1: Service Registration & Discovery

#### Service Registry
```
// Create registry
let registry = service_registry_create();

// Register services
let reg = service_register(registry, "api-service", "localhost", 8080);
let reg = service_register(reg, "db-service", "localhost", 5432);
let reg = service_register(reg, "cache-service", "localhost", 6379);

// Discover services
let api_endpoints = service_discover(reg, "api-service");
// Returns: [{"host": "localhost", "port": 8080}]

// Check health
let health = service_health_check("api-service");
// {"status": "healthy", "latency_ms": 45}

// Get service dependencies
let deps = service_get_dependencies("api-service");
```

### Chapter 2: Load Balancing & Resilience

#### Load Balancing
```
// Create load balancer
let lb = load_balancer_create("round_robin");

// Add servers
let lb = load_balancer_add_server(lb, "server1:8080");
let lb = load_balancer_add_server(lb, "server2:8080");
let lb = load_balancer_add_server(lb, "server3:8080");

// Select server (round-robin)
let server = load_balancer_select_server(lb);  // "server1:8080"
let server = load_balancer_select_server(lb);  // "server2:8080"
let server = load_balancer_select_server(lb);  // "server3:8080"

// Get statistics
let stats = load_balancer_get_stats(lb);
// {"total_requests": 1000, "avg_response_time_ms": 50}

// Different strategies
let least_conn = load_balancer_create("least_connections");
let random_lb = load_balancer_create("random");
let consistent = load_balancer_create("consistent_hash");
```

#### Circuit Breaker
```
// Create circuit breaker
let breaker = circuit_breaker_create(5, 30000);  // 5 failures, 30s timeout

// Execute with protection
let result = circuit_breaker_call(breaker, pub fn() -> String {
    http_get("https://api.example.com/data")
});

// Check state
if circuit_breaker_is_closed(breaker) then
    // Normal operation
else if circuit_breaker_is_open(breaker) then
    // Blocking calls - use fallback
    fallback_response()
else
    // Half-open - testing
end

// Record outcomes
circuit_breaker_record_success(breaker);
circuit_breaker_record_failure(breaker);

// Reset
circuit_breaker_reset(breaker);
```

#### Retry with Backoff
```
// Retry with exponential backoff
let data = retry_with_backoff(pub fn() -> String {
    http_get("https://api.example.com/data")
}, 3, 100);  // 3 retries, 100ms initial delay

// Timeout handling
let result = timeout_call(pub fn() -> String {
    long_running_operation()
}, 5000);  // 5 second timeout

// Timeout with fallback
let safe_result = timeout_call_default(pub fn() -> String {
    operation()
}, 1000, "default_value");  // 1 second timeout
```

### Chapter 3: Pub/Sub Messaging

#### Publishing & Subscribing
```
// Create topic
let topic = pub_sub_create("user-events");

// Subscribe to topic
let sub = pub_sub_subscribe(topic, pub fn(message: String) -> String {
    // Handle message
    log("Received: " + message);
    message
});

// Publish message
pub_sub_publish(topic, "{\"type\": \"user_created\", \"id\": 123}");

// Publish batch
let messages = "[\"msg1\", \"msg2\", \"msg3\"]";
pub_sub_publish_batch(topic, messages);

// Get subscriber count
let count = pub_sub_get_subscribers(topic);

// Subscriber group (load balanced)
let group_sub = pub_sub_subscribe_group(topic, "user-service", pub fn(msg: String) -> String {
    process_message(msg)
});
```

### Chapter 4: Event Streaming

#### Event Streams
```
// Create event stream
let stream = event_stream_create("transactions", 86400000);  // 24 hour TTL

// Add events
let stream = event_stream_add_event(stream, "{\"amount\": 100, \"from\": \"Alice\"}");
let stream = event_stream_add_event(stream, "{\"amount\": 50, \"from\": \"Bob\"}");

// Read events
let events = event_stream_read(stream, 0, 10);  // First 10 events

// Read all events
let all = event_stream_read_all(stream);

// Consumer groups (parallel consumption)
let group = consumer_group_create(stream, "payment-processor");
let batch = consumer_group_read(group, 5, 1000);  // Read 5, wait 1s timeout
```

---

## AXIOM Tutorial: Formal Verification

### Chapter 1: Type Safety

#### Dependent Types
```
// Create dependent type: "positive integers"
let positive_int = type_refine("integer", "value > 0");

// Check type
let is_valid = type_check(42, positive_int);  // 1 (valid)
let is_invalid = type_check(-5, positive_int);  // 0 (invalid)

// Refine types
let even_positive = type_refine(positive_int, "value % 2 == 0");

// Type unions
let number_type = type_union("integer", "float");

// Subtype checking
let is_subtype = type_is_subtype(positive_int, "integer");  // 1 (true)
```

### Chapter 2: Theorem Proving

#### Creating & Verifying Proofs
```
// Create theorem
let theorem = theorem_create(
    "addition_commutative",
    "a + b = b + a",
    "proof_term_here"
);

// Check if proven
let proven = theorem_is_proven(theorem);  // 1 if proved

// Verify proof
let verification = theorem_verify(theorem);

// Get proof
let proof = theorem_get_proof(theorem);

// Apply theorem
let result = theorem_apply(theorem, "{\"a\": 5, \"b\": 3}");
```

#### Using Lemmas & Axioms
```
// Create lemma
let lemma = lemma_create(
    "associativity",
    "(a + b) + c = a + (b + c)",
    ""
);

// Use lemma in proof
let proof = lemma_invoke(lemma, "{\"context\": \"addition_of_integers\"}");

// Declare axiom
let axiom = axiom_declare(
    "axiom_of_choice",
    "For any collection of sets, there exists a choice function"
);
```

### Chapter 3: Model Checking

#### Verifying System Properties
```
// Create model checker
let model = model_checking_create(
    "traffic_light_system",
    "G(red => X(yellow)) AND G(yellow => X(green))"
);  // Model and LTL properties

// Verify all properties
let verified = model_checking_verify(model);  // 1 if all pass

if verified == 0 then
    // Some property failed - get counterexample
    let counterexample = model_checking_get_counterexample(model);
    log("Counterexample: " + counterexample);
end

// Set timeout for verification
model_checking_set_timeout(model, 5000);  // 5 second timeout

// Get statistics
let stats = model_checker_get_stats(model);
// {"states_explored": 1000, "time_ms": 234, "sat": 1}
```

#### Temporal Logic
```
// LTL (Linear Temporal Logic) properties
let safety = temporal_logic_ltl("G(locked => X(unlocked))");  // Globally
let liveness = temporal_logic_ltl("F(request => F(response))");  // Eventually

// MTL (Metric Temporal Logic) with time bounds
let bounded = temporal_logic_mtl(
    "request => F[0,100](response)",
    "response within 100ms of request"
);

// Operators:
// G = Globally (always)
// F = Eventually (finally)
// X = Next
// U = Until
// [ ] = Always
// < > = Eventually
```

### Chapter 4: SMT Solving

#### Satisfiability Checking
```
// Create SMT solver
let solver = smt_solver_create("QF_LIA");  // Linear integer arithmetic

// Assert formulas
let solver = smt_solver_assert(solver, "x > 0");
let solver = smt_solver_assert(solver, "y > 0");
let solver = smt_solver_assert(solver, "x + y < 100");

// Check if satisfiable
let result = smt_solver_check_sat(solver);
// Returns "sat", "unsat", or "unknown"

if result then
    // Get model
    let model = smt_solver_get_model(solver);
    // {"x": 30, "y": 50}
end

// Context management
smt_solver_push(solver);
smt_solver_assert(solver, "x = 10");
if smt_solver_check_sat(solver) then
    // Use this model
end
smt_solver_pop(solver);  // Backtrack
```

---

## Complete Examples

### Example 1: Data Analysis Pipeline (TITAN → SYLVA → AETHER)

```
// 1. TITAN: Load CSV file
let csv_data = file_read_all("sales_data.csv");
file_write("raw_data.csv", csv_data);

// 2. SYLVA: Load and analyze data
let df = dataframe_from_csv("raw_data.csv");

// Clean data
let clean_df = dataframe_drop_missing(df);
let clean_df = dataframe_drop_duplicates(clean_df, "transaction_id");

// Analyze by region
let by_region = dataframe_groupby(clean_df, "region");
let region_sales = groupby_sum(by_region);

// Train model
let features = dataframe_select_columns(clean_df, "price,quantity,discount");
let labels = dataframe_select_columns(clean_df, "profit");
let model = random_forest(features, labels, 50);

// 3. AETHER: Deploy model as service
let service = sylva_model_to_aether_service(model, "sales-predictor", 8080);

// 4. TITAN: Log results
let results = json_stringify(service);
file_write("deployment_log.json", results);
```

### Example 2: Real-time Processing Pipeline (AETHER → SYLVA)

```
// 1. AETHER: Create event stream
let stream = event_stream_create("sensor-data", 3600000);  // 1 hour retention

// Simulate sensors sending data
pub_sub_publish("sensors", "{\"temperature\": 25.3, \"humidity\": 65}");

// 2. AETHER: Stream analytics
let analytics = sylva_analytics_on_aether_stream("sensors", 5000);

// 3. SYLVA: Real-time forecasting
let realtime = sylva_aether_realtime_pipeline(
    "sensor-input",
    trained_model,
    "predictions"
);

// 4. AETHER: Circuit breaker for reliability
let protected = aether_ml_service_with_circuit_breaker(model, 5);
```

### Example 3: Verified Smart Contract (TITAN → AXIOM)

```
// 1. TITAN: Write contract code
let contract = "
  fn transfer(from, to, amount) {
    require(balance[from] >= amount);
    balance[from] -= amount;
    balance[to] += amount;
  }
";

// 2. AXIOM: Formal verification
let spec = titan_code_to_axiom_formal_spec("contract.ti");

// Verify safety properties
let verified = theorem_verify(spec);

// 3. TITAN: If verified, deploy
if verified then
    file_write("verified_contract.sol", contract);
else
    let counterexample = model_checking_get_counterexample(spec);
    log("Contract unsafe: " + counterexample);
end
```

---

**Complete tutorials covering all 4 languages with practical examples and best practices.**

