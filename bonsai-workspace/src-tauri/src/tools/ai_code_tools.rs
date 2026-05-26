//! AI-assisted code intelligence tools.
//! Each tool builds a structured prompt and calls the local BonsAI inference
//! endpoint (port read from env or default 11434). Fully offline when a model is loaded.

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tool_registry::{Tool, ToolResult};

const DEFAULT_API: &str = "http://127.0.0.1:11434";

async fn call_local_llm(system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
    let api_base = std::env::var("BONSAI_API_URL")
        .unwrap_or_else(|_| DEFAULT_API.to_string());
    let url = format!("{api_base}/v1/chat/completions");
    let body = json!({
        "model": "bonsai",
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "max_tokens": max_tokens,
        "temperature": 0.3
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build().map_err(|e| e.to_string())?;
    let resp = client.post(&url).json(&body).send().await
        .map_err(|e| format!("LLM request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("LLM returned {}", resp.status()));
    }
    let data: Value = resp.json().await.map_err(|e| e.to_string())?;
    data["choices"][0]["message"]["content"].as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No content in LLM response".to_string())
}

// ── Code Explain ──────────────────────────────────────────────────────────────

pub struct CodeExplainTool;
#[async_trait]
impl Tool for CodeExplainTool {
    fn name(&self) -> &str { "code_explain" }
    fn description(&self) -> &str { "Explain what a code block does in plain language, including algorithmic complexity and design patterns." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code = args["code"].as_str().ok_or("Missing 'code'")?;
        let lang = args["language"].as_str().unwrap_or("auto-detect");
        let detail = args["detail"].as_str().unwrap_or("normal"); // brief|normal|detailed
        let system = "You are an expert software engineer. Explain code clearly and accurately. Include: what the code does, algorithmic complexity (Big-O), design patterns used, and any notable implementation details. Be precise and technical.";
        let user = format!("Language: {lang}\nDetail level: {detail}\n\nCode:\n```\n{code}\n```\n\nProvide a structured explanation.");
        let explanation = call_local_llm(system, &user, 1024).await?;
        Ok(ToolResult::json(&json!({ "explanation": explanation, "language": lang, "code_length": code.len() })))
    }
}

// ── Code Refactor ─────────────────────────────────────────────────────────────

pub struct CodeRefactorTool;
#[async_trait]
impl Tool for CodeRefactorTool {
    fn name(&self) -> &str { "code_refactor" }
    fn description(&self) -> &str { "Suggest and apply refactoring improvements: extract method, rename variables, simplify conditionals, reduce complexity." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code  = args["code"].as_str().ok_or("Missing 'code'")?;
        let lang  = args["language"].as_str().unwrap_or("auto-detect");
        let goals = args["goals"].as_str().unwrap_or("readability,maintainability,performance");
        let system = "You are a senior software engineer specializing in code quality. Return JSON with fields: 'refactored_code' (the improved code), 'changes' (array of {type, description, before, after}), 'complexity_before' (cyclomatic), 'complexity_after', 'summary'. Only return valid JSON.";
        let user = format!("Language: {lang}\nRefactoring goals: {goals}\n\nCode:\n```\n{code}\n```");
        let raw = call_local_llm(system, &user, 2048).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "refactored_code": raw, "changes": [], "summary": "Refactoring suggestions applied" }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate Tests ────────────────────────────────────────────────────────────

pub struct GenerateTestsTool;
#[async_trait]
impl Tool for GenerateTestsTool {
    fn name(&self) -> &str { "generate_tests" }
    fn description(&self) -> &str { "Generate comprehensive unit tests for a function with edge cases, happy path, error cases, and mocks." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code      = args["code"].as_str().ok_or("Missing 'code'")?;
        let lang      = args["language"].as_str().unwrap_or("auto-detect");
        let framework = args["framework"].as_str().unwrap_or("auto"); // jest/pytest/cargo-test/junit
        let system = format!("You are a test-driven development expert. Generate comprehensive unit tests in {lang} using {framework}. Include: happy path, edge cases (empty, null, boundary values), error/exception cases, and mocks for external dependencies. Follow the Arrange-Act-Assert pattern. Return only the test code.");
        let user = format!("Generate tests for:\n```{lang}\n{code}\n```");
        let tests = call_local_llm(&system, &user, 2048).await?;
        Ok(ToolResult::json(&json!({ "tests": tests, "language": lang, "framework": framework })))
    }
}

// ── Code Review ───────────────────────────────────────────────────────────────

pub struct CodeReviewTool;
#[async_trait]
impl Tool for CodeReviewTool {
    fn name(&self) -> &str { "code_review" }
    fn description(&self) -> &str { "Security-focused code review covering OWASP Top 10, injection, authentication, cryptography, input validation, and error handling." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code  = args["code"].as_str().ok_or("Missing 'code'")?;
        let lang  = args["language"].as_str().unwrap_or("auto-detect");
        let focus = args["focus"].as_str().unwrap_or("security,quality,performance");
        let system = "You are a senior security engineer conducting a code review. Analyze for: OWASP Top 10 vulnerabilities, SQL/command/XSS injection, authentication/authorization flaws, insecure cryptography, sensitive data exposure, missing input validation, error handling leaks, hardcoded secrets, race conditions. Return JSON: {\"severity\": \"critical|high|medium|low|info\", \"issues\": [{\"severity\", \"category\", \"line_hint\", \"description\", \"recommendation\"}], \"overall_score\": 0-10, \"summary\"}";
        let user = format!("Language: {lang}\nFocus areas: {focus}\n\nCode:\n```\n{code}\n```");
        let raw  = call_local_llm(system, &user, 2048).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "summary": raw, "issues": [], "overall_score": null }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate Documentation ────────────────────────────────────────────────────

pub struct GenerateDocumentationTool;
#[async_trait]
impl Tool for GenerateDocumentationTool {
    fn name(&self) -> &str { "generate_documentation" }
    fn description(&self) -> &str { "Generate comprehensive documentation: docstrings, README, API docs, or inline comments for a code file." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code   = args["code"].as_str().ok_or("Missing 'code'")?;
        let lang   = args["language"].as_str().unwrap_or("auto-detect");
        let format = args["format"].as_str().unwrap_or("docstring"); // docstring|readme|api_docs|inline
        let style  = args["style"].as_str().unwrap_or("google"); // google|numpy|jsdoc|rustdoc
        let system = format!("You are a technical writer and software engineer. Generate {format} documentation in {style} style for {lang} code. Be comprehensive but concise. Include: purpose, parameters with types, return values, examples, exceptions/errors, and complexity notes where relevant.");
        let user = format!("Generate {format} documentation ({style} style) for:\n```{lang}\n{code}\n```");
        let docs = call_local_llm(&system, &user, 2048).await?;
        Ok(ToolResult::json(&json!({ "documentation": docs, "format": format, "style": style, "language": lang })))
    }
}

// ── Dependency Audit ──────────────────────────────────────────────────────────

pub struct DependencyAuditTool;
#[async_trait]
impl Tool for DependencyAuditTool {
    fn name(&self) -> &str { "dependency_audit" }
    fn description(&self) -> &str { "Audit project dependencies for outdated packages, license issues, and known vulnerability patterns by parsing manifest files." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path = args["path"].as_str().unwrap_or(".");
        let base = std::path::Path::new(path);
        let mut findings = Vec::new();

        // Detect and parse manifest files
        let manifests = [
            ("Cargo.toml", "rust"),
            ("package.json", "node"),
            ("requirements.txt", "python"),
            ("go.mod", "go"),
            ("pom.xml", "java"),
            ("Gemfile", "ruby"),
        ];
        for (file, lang) in &manifests {
            let p = base.join(file);
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    let deps = parse_deps_from_manifest(&content, lang);
                    findings.push(json!({ "manifest": file, "language": lang, "dependencies": deps, "count": deps.len() }));
                }
            }
        }

        // LLM-assisted analysis of found dependencies
        if !findings.is_empty() {
            let deps_summary = serde_json::to_string_pretty(&findings).unwrap_or_default();
            let system = "You are a security expert. Analyze these dependencies for: known problematic packages, suspicious names (typosquatting), overly broad permissions, deprecated packages, and license compliance risks. Return JSON: {\"risks\": [{\"package\", \"type\", \"severity\", \"description\"}], \"recommendation\", \"total_packages\"}";
            let user = format!("Audit these project dependencies:\n{deps_summary}");
            if let Ok(analysis) = call_local_llm(system, &user, 1024).await {
                let parsed: Value = serde_json::from_str(&analysis).unwrap_or(json!({ "summary": analysis }));
                return Ok(ToolResult::json(&json!({ "manifests": findings, "analysis": parsed })));
            }
        }
        Ok(ToolResult::json(&json!({ "manifests": findings, "message": "Scan complete" })))
    }
}

fn parse_deps_from_manifest(content: &str, lang: &str) -> Vec<Value> {
    match lang {
        "rust" => {
            // Simple Cargo.toml dep extraction
            let mut deps = Vec::new();
            let mut in_deps = false;
            for line in content.lines() {
                let t = line.trim();
                if t == "[dependencies]" || t == "[dev-dependencies]" { in_deps = true; continue; }
                if t.starts_with('[') { in_deps = false; }
                if in_deps && t.contains('=') {
                    if let Some((name, ver)) = t.split_once('=') {
                        deps.push(json!({ "name": name.trim(), "version": ver.trim().trim_matches('"') }));
                    }
                }
            }
            deps
        }
        "python" => content.lines()
            .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
            .map(|l| { let (name, ver) = l.split_once(&['=','>', '<'][..]).unwrap_or((l, "")); json!({ "name": name.trim(), "version": ver.trim() }) })
            .collect(),
        _ => content.lines().take(50)
            .filter(|l| !l.trim().is_empty())
            .map(|l| json!({ "line": l.trim() }))
            .collect(),
    }
}

// ── Generate API Client ───────────────────────────────────────────────────────

pub struct GenerateApiClientTool;
#[async_trait]
impl Tool for GenerateApiClientTool {
    fn name(&self) -> &str { "generate_api_client" }
    fn description(&self) -> &str { "Generate a typed API client (Rust/Python/TypeScript) from an OpenAPI spec, curl command, or API description." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let spec     = args["spec"].as_str().ok_or("Missing 'spec' (OpenAPI JSON/YAML, curl command, or description)")?;
        let language = args["language"].as_str().unwrap_or("typescript");
        let style    = args["style"].as_str().unwrap_or("async"); // sync|async
        let system = format!("You are an expert API engineer. Generate a complete, production-ready {language} API client ({style} style) from the provided specification. Include: typed interfaces/structs for all request/response types, error handling, retry logic, and usage examples. Follow idiomatic {language} patterns.");
        let user = format!("Generate a {language} API client for:\n\n{spec}");
        let client_code = call_local_llm(&system, &user, 3000).await?;
        Ok(ToolResult::json(&json!({ "client_code": client_code, "language": language, "style": style })))
    }
}

// ── Migrate Code ──────────────────────────────────────────────────────────────

pub struct MigrateCodeTool;
#[async_trait]
impl Tool for MigrateCodeTool {
    fn name(&self) -> &str { "migrate_code" }
    fn description(&self) -> &str { "Convert code between frameworks or languages (e.g., Express→Fastify, Python→Rust, callbacks→async/await)." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let code      = args["code"].as_str().ok_or("Missing 'code'")?;
        let from      = args["from"].as_str().ok_or("Missing 'from' (source language/framework)")?;
        let to        = args["to"].as_str().ok_or("Missing 'to' (target language/framework)")?;
        let preserve  = args["preserve"].as_str().unwrap_or("logic,behavior,naming");
        let system = format!("You are an expert polyglot programmer. Migrate code from {from} to {to} idiomatically. Preserve: {preserve}. Return JSON: {{\"migrated_code\", \"notes\": [\"migration note\"], \"breaking_changes\": [], \"manual_steps\": []}}");
        let user = format!("Migrate from {from} to {to}:\n```\n{code}\n```");
        let raw = call_local_llm(&system, &user, 3000).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "migrated_code": raw, "notes": [] }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate CI Pipeline ──────────────────────────────────────────────────────

pub struct GenerateCiPipelineTool;
#[async_trait]
impl Tool for GenerateCiPipelineTool {
    fn name(&self) -> &str { "generate_ci_pipeline" }
    fn description(&self) -> &str { "Generate a GitHub Actions, GitLab CI, or Jenkins pipeline for the current project based on detected languages and frameworks." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let project_dir = args["path"].as_str().unwrap_or(".");
        let platform    = args["platform"].as_str().unwrap_or("github_actions"); // github_actions|gitlab_ci|jenkins
        let features    = args["features"].as_str().unwrap_or("lint,test,build,deploy");

        // Detect project type
        let base = std::path::Path::new(project_dir);
        let mut detected = Vec::new();
        for (file, lang) in &[("Cargo.toml","rust"),("package.json","node"),("requirements.txt","python"),("go.mod","go"),("pom.xml","java")] {
            if base.join(file).exists() { detected.push(*lang); }
        }

        let system = format!("You are a DevOps engineer. Generate a {platform} CI/CD pipeline configuration that: builds the project, runs all tests, checks code style/linting, and optionally deploys. Include caching for dependencies, parallel jobs where possible, and security scanning. Return only the YAML/Groovy configuration file content.");
        let user = format!("Project languages/frameworks detected: {}\nFeatures to include: {features}\nPlatform: {platform}\n\nGenerate the CI pipeline configuration.", detected.join(", "));
        let pipeline = call_local_llm(&system, &user, 2048).await?;
        let filename = match platform {
            "github_actions" => ".github/workflows/ci.yml",
            "gitlab_ci"      => ".gitlab-ci.yml",
            "jenkins"        => "Jenkinsfile",
            _                => "ci.yml",
        };
        Ok(ToolResult::json(&json!({ "pipeline": pipeline, "platform": platform, "filename": filename, "detected_languages": detected })))
    }
}

// ── Generate Commit Message ───────────────────────────────────────────────────

pub struct GenerateCommitMessageTool;
#[async_trait]
impl Tool for GenerateCommitMessageTool {
    fn name(&self) -> &str { "generate_commit_message" }
    fn description(&self) -> &str { "Generate a conventional commit message from a git diff or change description." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let diff_or_desc = args["diff"].as_str()
            .or_else(|| args["description"].as_str())
            .ok_or("Missing 'diff' or 'description'")?;
        let style = args["style"].as_str().unwrap_or("conventional"); // conventional|simple|detailed
        let scope = args["scope"].as_str().unwrap_or("");

        let system = "You are an expert at writing clear, concise git commit messages. Follow Conventional Commits specification (feat/fix/docs/style/refactor/test/chore). Return JSON: {\"subject\": \"type(scope): description\", \"body\": \"optional extended description\", \"breaking\": false, \"type\": \"feat\", \"scope\": \"...\"}";
        let user = format!("Style: {style}\nScope hint: {scope}\n\nChanges:\n{diff_or_desc}");
        let raw = call_local_llm(system, &user, 512).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "subject": raw.lines().next().unwrap_or(&raw), "body": "" }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Optimize Prompt ───────────────────────────────────────────────────────────

pub struct OptimizePromptTool;
#[async_trait]
impl Tool for OptimizePromptTool {
    fn name(&self) -> &str { "optimize_prompt" }
    fn description(&self) -> &str { "Analyze and optimize an LLM prompt for clarity, specificity, structure, and expected output quality." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt  = args["prompt"].as_str().ok_or("Missing 'prompt'")?;
        let goal    = args["goal"].as_str().unwrap_or("general");
        let model   = args["model"].as_str().unwrap_or("general");
        let system = "You are a prompt engineering expert. Analyze the given prompt and return JSON: {\"optimized_prompt\": \"...\", \"issues\": [\"vague instruction\", \"missing context\"], \"improvements\": [{\"issue\", \"fix\"}], \"clarity_score\": 0-10, \"specificity_score\": 0-10, \"estimated_improvement\": \"...\"}";
        let user = format!("Goal: {goal}\nTarget model type: {model}\n\nOriginal prompt:\n{prompt}");
        let raw = call_local_llm(system, &user, 1024).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "optimized_prompt": raw, "issues": [] }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── SQL Explain ───────────────────────────────────────────────────────────────

pub struct SqlExplainTool;
#[async_trait]
impl Tool for SqlExplainTool {
    fn name(&self) -> &str { "sql_explain" }
    fn description(&self) -> &str { "Explain a SQL query with execution plan analysis, optimization suggestions, and complexity estimate." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let sql    = args["sql"].as_str().ok_or("Missing 'sql'")?;
        let schema = args["schema"].as_str().unwrap_or("");
        let dialect = args["dialect"].as_str().unwrap_or("standard"); // postgres|mysql|sqlite|standard
        let system = "You are a database expert. Analyze this SQL query and return JSON: {\"plain_english\": \"what this query does\", \"steps\": [\"step 1\"], \"complexity\": \"O(n)\", \"bottlenecks\": [], \"optimizations\": [{\"issue\", \"suggestion\", \"rewritten_query\"}], \"indexes_needed\": [], \"estimated_cost\": \"low|medium|high\"}";
        let user = format!("Dialect: {dialect}\nSchema context:\n{schema}\n\nSQL:\n{sql}");
        let raw = call_local_llm(system, &user, 1024).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "plain_english": raw, "optimizations": [] }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Query Builder ─────────────────────────────────────────────────────────────

pub struct QueryBuilderTool;
#[async_trait]
impl Tool for QueryBuilderTool {
    fn name(&self) -> &str { "query_builder" }
    fn description(&self) -> &str { "Convert natural language to SQL with proper joins, aggregations, and window functions given a schema." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let question = args["question"].as_str().ok_or("Missing 'question'")?;
        let schema   = args["schema"].as_str().unwrap_or("");
        let dialect  = args["dialect"].as_str().unwrap_or("standard");
        let system = format!("You are a SQL expert. Convert the natural language question to {dialect} SQL. Return JSON: {{\"sql\": \"SELECT ...\", \"explanation\": \"...\", \"tables_used\": [], \"assumptions\": []}}");
        let user = format!("Schema:\n{schema}\n\nQuestion: {question}");
        let raw = call_local_llm(&system, &user, 1024).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "sql": raw, "explanation": "" }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Database Diagram ──────────────────────────────────────────────────────────

pub struct DatabaseDiagramTool;
#[async_trait]
impl Tool for DatabaseDiagramTool {
    fn name(&self) -> &str { "database_diagram" }
    fn description(&self) -> &str { "Generate an ER diagram in Mermaid format from a SQL schema or table definitions." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let schema  = args["schema"].as_str().ok_or("Missing 'schema'")?;
        let format  = args["format"].as_str().unwrap_or("mermaid"); // mermaid|plantuml
        let system = format!("You are a database architect. Generate a {format} ER diagram from the SQL schema. Include all tables, columns (with types), primary keys, and foreign key relationships. Return only the diagram code.");
        let user   = format!("Generate {format} ER diagram for:\n{schema}");
        let diagram = call_local_llm(&system, &user, 2048).await?;
        Ok(ToolResult::json(&json!({ "diagram": diagram, "format": format })))
    }
}

// ── Summarize Meeting Notes ───────────────────────────────────────────────────

pub struct SummarizeMeetingNotesTool;
#[async_trait]
impl Tool for SummarizeMeetingNotesTool {
    fn name(&self) -> &str { "summarize_meeting_notes" }
    fn description(&self) -> &str { "Convert a raw meeting transcript into structured notes with action items, decisions, open questions, and key topics." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let transcript = args["transcript"].as_str().ok_or("Missing 'transcript'")?;
        let attendees  = args["attendees"].as_str().unwrap_or("");
        let context    = args["context"].as_str().unwrap_or("");
        let system = "You are an expert meeting facilitator. Analyze the transcript and return JSON: {\"title\": \"...\", \"date\": \"...\", \"summary\": \"2-3 sentence overview\", \"action_items\": [{\"owner\", \"task\", \"due_date\"}], \"decisions\": [\"...\"], \"open_questions\": [\"...\"], \"key_topics\": [\"...\"], \"next_steps\": [\"...\"]}";
        let user = format!("Attendees: {attendees}\nContext: {context}\n\nTranscript:\n{transcript}");
        let raw = call_local_llm(system, &user, 1500).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "summary": raw, "action_items": [] }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate Email ────────────────────────────────────────────────────────────

pub struct GenerateEmailTool;
#[async_trait]
impl Tool for GenerateEmailTool {
    fn name(&self) -> &str { "generate_email" }
    fn description(&self) -> &str { "Generate a professional email from bullet points with tone options (formal, casual, persuasive, apologetic)." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let points   = args["points"].as_str().ok_or("Missing 'points' (bullet points or description)")?;
        let tone     = args["tone"].as_str().unwrap_or("professional"); // professional|casual|persuasive|apologetic|assertive
        let to       = args["to"].as_str().unwrap_or("");
        let subject_hint = args["subject"].as_str().unwrap_or("");
        let system = format!("You are an expert business communicator. Write an email with {tone} tone. Return JSON: {{\"subject\": \"...\", \"body\": \"...\", \"call_to_action\": \"...\", \"estimated_read_time\": \"X seconds\"}}");
        let user = format!("To: {to}\nSubject hint: {subject_hint}\nTone: {tone}\n\nKey points to cover:\n{points}");
        let raw = call_local_llm(&system, &user, 1024).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "subject": subject_hint, "body": raw }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate Flashcards ───────────────────────────────────────────────────────

pub struct GenerateFlashcardsTool;
#[async_trait]
impl Tool for GenerateFlashcardsTool {
    fn name(&self) -> &str { "generate_flashcards" }
    fn description(&self) -> &str { "Generate Anki-compatible flashcards from study material with question, answer, and difficulty rating." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let material = args["material"].as_str().ok_or("Missing 'material'")?;
        let count    = args["count"].as_u64().unwrap_or(10).min(50) as usize;
        let style    = args["style"].as_str().unwrap_or("qa"); // qa|cloze|definition
        let system = format!("You are an expert educator using spaced repetition. Generate {count} {style} flashcards from the material. Return JSON: {{\"cards\": [{{\"front\": \"...\", \"back\": \"...\", \"difficulty\": \"easy|medium|hard\", \"tags\": []}}], \"topic\": \"...\", \"total_cards\": {count}}}");
        let user = format!("Style: {style}\nCards needed: {count}\n\nStudy material:\n{material}");
        let raw = call_local_llm(&system, &user, 2048).await?;
        let mut parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "cards": [], "topic": "Unknown" }));

        // Also generate CSV for Anki import
        if let Some(cards) = parsed["cards"].as_array() {
            let csv: String = cards.iter().map(|c| {
                let front = c["front"].as_str().unwrap_or("");
                let back  = c["back"].as_str().unwrap_or("");
                format!("{};{}", front.replace(';', ","), back.replace(';', ","))
            }).collect::<Vec<_>>().join("\n");
            parsed["anki_csv"] = Value::String(csv);
        }
        Ok(ToolResult::json(&parsed))
    }
}

// ── Grammar Check ─────────────────────────────────────────────────────────────

pub struct GrammarCheckTool;
#[async_trait]
impl Tool for GrammarCheckTool {
    fn name(&self) -> &str { "grammar_check" }
    fn description(&self) -> &str { "Check grammar, style, and clarity with specific suggestions (passive voice, wordiness, readability, consistency)." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let text  = args["text"].as_str().ok_or("Missing 'text'")?;
        let style = args["style"].as_str().unwrap_or("professional"); // professional|academic|casual
        let lang  = args["language"].as_str().unwrap_or("en-US");

        // Rule-based pre-check
        let mut quick_issues = Vec::new();
        let lower = text.to_lowercase();
        if lower.contains(" irregardless ") { quick_issues.push("'irregardless' is not standard — use 'regardless'"); }
        if lower.contains(" alot ") { quick_issues.push("'alot' should be 'a lot'"); }
        if lower.contains(" your welcome") { quick_issues.push("'your welcome' should be 'you're welcome'"); }

        // Count passive voice indicators
        let passive = ["was ", "were ", "been ", "being ", "is ", "are "].iter()
            .map(|p| text.matches(p).count()).sum::<usize>();

        let system = format!("You are a professional editor. Review the text for grammar, spelling, style, and clarity. Language: {lang}, Style guide: {style}. Return JSON: {{\"corrected_text\": \"...\", \"issues\": [{{\"type\": \"grammar|spelling|style|clarity\", \"original\": \"...\", \"suggestion\": \"...\", \"explanation\": \"...\"}}], \"readability_score\": 0-10, \"passive_voice_count\": N, \"summary\": \"...\"}}");
        let user = format!("Style: {style}\nLanguage: {lang}\n\nText:\n{text}");
        let raw = call_local_llm(&system, &user, 2048).await?;
        let mut parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "corrected_text": text, "issues": [] }));
        parsed["quick_issues"] = json!(quick_issues);
        parsed["passive_voice_indicators"] = json!(passive);
        Ok(ToolResult::json(&parsed))
    }
}

// ── Persona Switch ────────────────────────────────────────────────────────────

pub struct PersonaSwitchTool;
#[async_trait]
impl Tool for PersonaSwitchTool {
    fn name(&self) -> &str { "persona_switch" }
    fn description(&self) -> &str { "Switch BonsAI's response style: teacher, critic, rubber_duck, mentor, peer, socratic, devil_advocate, executive." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let persona = args["persona"].as_str().ok_or("Missing 'persona'")?;
        let content = args["content"].as_str().ok_or("Missing 'content' to respond to")?;
        let system = match persona {
            "teacher"         => "You are a patient, encouraging teacher. Break down concepts clearly with examples, check for understanding, and build from fundamentals. Use analogies and ask questions to gauge comprehension.",
            "critic"          => "You are a rigorous critic. Find flaws, weaknesses, and counter-arguments. Be direct but constructive. Challenge assumptions and push for higher standards.",
            "rubber_duck"     => "You are a rubber duck debugger. Listen carefully and ask clarifying questions: 'What exactly does this line do?', 'What do you expect to happen?', 'Have you checked X?'. Help the user think through their own problem.",
            "mentor"          => "You are a senior mentor with deep experience. Share wisdom from experience, explain tradeoffs, suggest resources, and help build long-term skills rather than just solving the immediate problem.",
            "peer"            => "You are a knowledgeable peer programmer. Collaborate as an equal, brainstorm together, share your own experiences, and work through problems side by side.",
            "socratic"        => "You only ask questions — never give direct answers. Guide through probing questions that lead the user to discover the answer themselves.",
            "devil_advocate"  => "You argue the opposite position from whatever the user presents. Find the strongest counter-arguments and edge cases.",
            "executive"       => "You are a C-level executive. Focus on business impact, ROI, risk, and strategic implications. Be concise. Use bullet points. Think in terms of outcomes, not implementation details.",
            _                 => "You are a helpful, versatile AI assistant.",
        };
        let response = call_local_llm(system, content, 1024).await?;
        Ok(ToolResult::json(&json!({ "persona": persona, "response": response })))
    }
}

// ── Context Snapshot ──────────────────────────────────────────────────────────

pub struct ContextSnapshotTool;
#[async_trait]
impl Tool for ContextSnapshotTool {
    fn name(&self) -> &str { "context_snapshot" }
    fn description(&self) -> &str { "Save or load a named conversation context snapshot to ~/.bonsai/snapshots/ for later continuation." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let name      = args["name"].as_str().ok_or("Missing 'name'")?;
        let operation = args["operation"].as_str().unwrap_or("save"); // save|load|list|delete
        let dir = dirs::home_dir().unwrap_or_default().join(".bonsai/snapshots");
        tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;

        match operation {
            "save" => {
                let content = args.get("content").ok_or("Missing 'content' to save")?;
                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                let snap = json!({ "name": name, "saved_at": ts, "content": content });
                let path  = dir.join(format!("{}.json", sanitize_filename(name)));
                tokio::fs::write(&path, serde_json::to_string_pretty(&snap).unwrap_or_default()).await.map_err(|e| e.to_string())?;
                Ok(ToolResult::json(&json!({ "saved": true, "path": path.to_string_lossy(), "name": name })))
            }
            "load" => {
                let path = dir.join(format!("{}.json", sanitize_filename(name)));
                let raw  = tokio::fs::read_to_string(&path).await.map_err(|_| format!("Snapshot '{}' not found", name))?;
                let snap: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                Ok(ToolResult::json(&snap))
            }
            "list" => {
                let mut entries = Vec::new();
                let mut dir_read = tokio::fs::read_dir(&dir).await.map_err(|e| e.to_string())?;
                while let Ok(Some(entry)) = dir_read.next_entry().await {
                    let fname = entry.file_name().to_string_lossy().to_string();
                    if fname.ends_with(".json") { entries.push(fname.trim_end_matches(".json").to_string()); }
                }
                Ok(ToolResult::json(&json!({ "snapshots": entries })))
            }
            "delete" => {
                let path = dir.join(format!("{}.json", sanitize_filename(name)));
                tokio::fs::remove_file(&path).await.map_err(|e| e.to_string())?;
                Ok(ToolResult::json(&json!({ "deleted": true, "name": name })))
            }
            _ => Err(format!("Unknown operation: {operation}")),
        }
    }
}

fn sanitize_filename(s: &str) -> String {
    s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

// ── Simulate Conversation ─────────────────────────────────────────────────────

pub struct SimulateConversationTool;
#[async_trait]
impl Tool for SimulateConversationTool {
    fn name(&self) -> &str { "simulate_conversation" }
    fn description(&self) -> &str { "Simulate a multi-turn conversation between two personas (e.g., customer and support agent, interviewer and candidate)." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let persona_a = args["persona_a"].as_str().ok_or("Missing 'persona_a'")?;
        let persona_b = args["persona_b"].as_str().ok_or("Missing 'persona_b'")?;
        let scenario  = args["scenario"].as_str().ok_or("Missing 'scenario'")?;
        let turns     = args["turns"].as_u64().unwrap_or(6).min(20) as usize;
        let system = format!("You are a skilled creative writer. Simulate a realistic {turns}-turn conversation between: Persona A ({persona_a}) and Persona B ({persona_b}). Scenario: {scenario}. Make each line feel authentic and distinct. Return JSON: {{\"conversation\": [{{\"speaker\": \"A\"|\"B\", \"text\": \"...\"}}], \"outcome\": \"brief summary of how it resolved\"}}");
        let user = format!("Simulate the conversation for this scenario: {scenario}");
        let raw = call_local_llm(&system, &user, 2048).await?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({ "conversation": [{"speaker": "A", "text": raw}] }));
        Ok(ToolResult::json(&parsed))
    }
}

// ── Generate Tutorial ─────────────────────────────────────────────────────────

pub struct GenerateTutorialTool;
#[async_trait]
impl Tool for GenerateTutorialTool {
    fn name(&self) -> &str { "generate_tutorial" }
    fn description(&self) -> &str { "Generate a structured, progressive tutorial for a topic with steps, code examples, and exercises." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let topic      = args["topic"].as_str().ok_or("Missing 'topic'")?;
        let level      = args["level"].as_str().unwrap_or("beginner"); // beginner|intermediate|advanced
        let format     = args["format"].as_str().unwrap_or("markdown"); // markdown|json
        let lang       = args["language"].as_str().unwrap_or("");
        let system = format!("You are an expert educator. Create a comprehensive {level} tutorial for '{topic}'. Include: learning objectives, prerequisites, step-by-step sections with explanations, code examples in {lang}, common mistakes to avoid, exercises, and next steps. Format as {format}.");
        let user = format!("Create a {level} tutorial for: {topic}");
        let tutorial = call_local_llm(&system, &user, 3000).await?;
        if format == "json" {
            let parsed: Value = serde_json::from_str(&tutorial).unwrap_or(json!({ "content": tutorial }));
            Ok(ToolResult::json(&parsed))
        } else {
            Ok(ToolResult::json(&json!({ "tutorial": tutorial, "topic": topic, "level": level })))
        }
    }
}

// ── Registration ──────────────────────────────────────────────────────────────

use std::sync::Arc;
use crate::tool_registry::Tool as ToolTrait;

pub fn all_ai_code_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        Arc::new(CodeExplainTool),
        Arc::new(CodeRefactorTool),
        Arc::new(GenerateTestsTool),
        Arc::new(CodeReviewTool),
        Arc::new(GenerateDocumentationTool),
        Arc::new(DependencyAuditTool),
        Arc::new(GenerateApiClientTool),
        Arc::new(MigrateCodeTool),
        Arc::new(GenerateCiPipelineTool),
        Arc::new(GenerateCommitMessageTool),
        Arc::new(OptimizePromptTool),
        Arc::new(SqlExplainTool),
        Arc::new(QueryBuilderTool),
        Arc::new(DatabaseDiagramTool),
        Arc::new(SummarizeMeetingNotesTool),
        Arc::new(GenerateEmailTool),
        Arc::new(GenerateFlashcardsTool),
        Arc::new(GrammarCheckTool),
        Arc::new(PersonaSwitchTool),
        Arc::new(ContextSnapshotTool),
        Arc::new(SimulateConversationTool),
        Arc::new(GenerateTutorialTool),
    ]
}
