# Data Framework Guide - Data Processing & Analytics

**Complete guide to data processing, transformation, and analytics**

---

## Overview

The Data Framework provides:
- **DataFrames**: Tabular data manipulation
- **ETL**: Extract, Transform, Load pipelines
- **Analytics**: Statistical analysis and aggregations
- **Time Series**: Temporal data operations
- **Serialization**: Multiple data formats

---

## DataFrames

### Creating DataFrames

```titan
use omnisystem::data::*

fun create_df() -> Result<DataFrame, str> {
    let df = DataFrame::new()
        .add_column("name", vec!["Alice", "Bob", "Charlie"])
        .add_column("age", vec![30, 25, 35])
        .add_column("salary", vec![50000.0, 45000.0, 60000.0])
    
    Ok(df)
}
```

### Loading Data

```titan
// CSV
let df = DataFrame::from_csv("data.csv")?

// JSON
let df = DataFrame::from_json("data.json")?

// SQL
let df = DataFrame::from_sql("SELECT * FROM users")?

// Parquet
let df = DataFrame::from_parquet("data.parquet")?
```

---

## Data Transformation

### Selection

```titan
// Select columns
let selected = df.select(&["name", "age"])?

// Select rows
let filtered = df.filter(|row| {
    row.get_i64("age")? > 25
})?

// Slice
let slice = df.slice(0, 10)?  // First 10 rows
```

### Aggregation

```titan
// Group by
let grouped = df.group_by("department")?
    .agg(AggFunc::Sum("salary"))
    .agg(AggFunc::Count("*"))

// Aggregations
let total = df.sum("salary")?
let avg = df.mean("salary")?
let max = df.max("age")?
let min = df.min("age")?
let count = df.count()?
```

### Transformation

```titan
// Map columns
let transformed = df.map("salary", |val| {
    (val as f64) * 1.1  // 10% raise
})?

// Add computed column
let with_bonus = df.add_column("bonus", |row| {
    row.get_f64("salary")? * 0.1
})?

// Rename columns
let renamed = df.rename("age", "years")?

// Join
let df1 = load_users()?
let df2 = load_departments()?
let joined = df1.join(&df2, "dept_id")?
```

---

## Analytics

### Descriptive Statistics

```titan
let stats = df.describe()?
// Shows: count, mean, std, min, 25%, 50%, 75%, max

let correlation = df.correlation()?
// Shows correlation matrix between numeric columns

let missing = df.missing_values()?
// Shows count of null values per column
```

### Distribution Analysis

```titan
let histogram = df.histogram("age", bins: 10)?

let distribution = df.value_counts("department")?
// Shows frequency of each value

let percentiles = df.percentiles("salary", &[25, 50, 75, 90, 95, 99])?
```

---

## ETL Pipeline

### Complete ETL Example

```titan
fun etl_pipeline() -> Result<DataFrame, str> {
    // Extract
    let raw = extract_from_source()?
    println!("Extracted {} rows", raw.count()?)
    
    // Transform
    let cleaned = raw
        .filter(|row| row.get_string("status")? == "active")?
        .map("amount", |val| (val as f64) * 1.1)?
        .add_column("processed_date", get_date)?
    
    println!("Transformed {} rows", cleaned.count()?)
    
    // Load
    load_to_warehouse(&cleaned)?
    println!("Loaded to warehouse")
    
    Ok(cleaned)
}

fn extract_from_source() -> Result<DataFrame, str> {
    DataFrame::from_csv("raw_data.csv")
}

fn load_to_warehouse(df: &DataFrame) -> Result<(), str> {
    df.to_sql("INSERT INTO processed_data ...")
}
```

---

## Time Series

### Time Series Operations

```titan
let ts = TimeSeries::new()
    .add("2026-01-01", 100.0)
    .add("2026-01-02", 102.0)
    .add("2026-01-03", 101.0)
    .add("2026-01-04", 105.0)

// Resampling
let daily = ts.resample("1D")?     // Daily data
let weekly = ts.resample("1W")?    // Weekly data
let monthly = ts.resample("1M")?   // Monthly data

// Rolling window
let ma_5 = ts.rolling_mean(window: 5)?    // 5-day moving average
let volatility = ts.rolling_std(window: 20)?

// Lag and shift
let lagged = ts.lag(1)?      // Previous day's value
let diff = ts.diff()?        // Day-over-day change

// Forecasting
let forecast = ts.forecast(periods: 30)?
```

---

## Data Quality

### Validation

```titan
let validator = DataValidator::new()
    .add_rule("age", Rule::InRange(0, 150))
    .add_rule("email", Rule::Matches(EMAIL_REGEX))
    .add_rule("salary", Rule::NotNull)

let errors = validator.validate(&df)?

if !errors.is_empty() {
    println!("Found {} validation errors", errors.len())
    for error in &errors {
        println!("Row {}: {}", error.row, error.message)
    }
}
```

### Data Cleaning

```titan
// Remove duplicates
let unique = df.drop_duplicates()?

// Handle missing values
let filled = df.fill_missing("age", 30)?
let dropped = df.drop_null_rows()?
let interpolated = df.interpolate("salary")?

// Normalize values
let normalized = df.normalize("salary", min: 0.0, max: 1.0)?

// Standardize
let standardized = df.standardize("age")?
```

---

## Advanced Analytics

### Correlation Analysis

```titan
let corr = df.correlation_with("salary", &["age", "experience"])?
// Shows correlation between salary and other variables

let significant = df.find_correlations(min_threshold: 0.7)?
// Shows all pairs with correlation > 0.7
```

### Outlier Detection

```titan
let outliers = df.detect_outliers("salary", method: "iqr")?
// Interquartile range method

let outliers = df.detect_outliers("price", method: "zscore")?
// Z-score method (standard deviations from mean)

let cleaned = df.remove_outliers("salary")?
```

### Dimensionality Reduction

```titan
let pca = df.pca(n_components: 2)?
// Reduce to 2 principal components
```

---

## Data Export

### Multiple Formats

```titan
// CSV
df.to_csv("output.csv")?

// JSON
df.to_json("output.json")?

// Parquet
df.to_parquet("output.parquet")?

// SQL Insert
df.to_sql("INSERT INTO users VALUES ...")?

// Excel
df.to_excel("output.xlsx")?

// HTML Table
df.to_html("output.html")?
```

---

## Performance

### Large Datasets

```titan
// Lazy evaluation
let result = df
    .filter(|row| row.get_i64("age")? > 25)
    .map("salary", |val| (val as f64) * 1.1)
    .collect()?  // Only execute when needed

// Chunked processing
let chunks = df.chunk(size: 1000)
for chunk in chunks {
    process_chunk(&chunk)?
}

// Parallel processing
let result = df.par_map("amount", |val| expensive_calc(val))?
```

---

## Example: Sales Analytics

```titan
use omnisystem::data::*

fun sales_analysis() -> Result<(), str> {
    // Load sales data
    let df = DataFrame::from_csv("sales.csv")?
    
    println!("Total sales: {}", df.sum("amount")?);
    
    // Sales by region
    let by_region = df.group_by("region")?
        .agg(AggFunc::Sum("amount"))
        .agg(AggFunc::Count("*"))?
    
    println!("\nSales by Region:");
    by_region.print()?
    
    // Top products
    let top_products = df.group_by("product")?
        .agg(AggFunc::Count("*"))?
        .sort_by("count", ascending: false)?
        .slice(0, 10)?
    
    println!("\nTop 10 Products:");
    top_products.print()?
    
    // Trend analysis
    let daily_sales = df.group_by("date")?
        .agg(AggFunc::Sum("amount"))?
    
    let ma = daily_sales.rolling_mean(window: 7)?
    
    println!("\n7-day Moving Average:");
    ma.print()?
    
    // Export results
    by_region.to_csv("sales_by_region.csv")?
    top_products.to_csv("top_products.csv")?
    
    Ok(())
}
```

---

## Integration with ML

### DataFrame to Tensor

```titan
use omnisystem::data::*
use omnisystem::sylva::*

fun df_to_tensor(df: &DataFrame) -> Result<Tensor, str> {
    let data = df.to_matrix()?
    let tensor = Tensor::from_matrix(&data)?
    Ok(tensor)
}

fun prepare_ml_data(df: &DataFrame) -> Result<(Tensor, Tensor), str> {
    // Feature columns
    let features = df.select(&["age", "experience", "salary"])?
        .normalize("age", 0.0, 1.0)?
        .to_tensor()?
    
    // Target column
    let target = df.select(&["promoted"])?
        .to_tensor()?
    
    Ok((features, target))
}
```

---

## Best Practices

✅ **DO**
- Validate data before processing
- Document transformations
- Version your datasets
- Test on sample data first
- Monitor data quality

❌ **DON'T**
- Modify raw data
- Skip validation
- Hardcode column names
- Process all data at once
- Ignore missing values

---

## Next Steps

- Integration: [LANGUAGE_BRIDGES.md](LANGUAGE_BRIDGES.md)
- ML Pipeline: [TUTORIAL_ML_AI.md](TUTORIAL_ML_AI.md)
- Performance: [PERFORMANCE.md](PERFORMANCE.md)

---

**Data Framework** - Transform and analyze data effortlessly!
