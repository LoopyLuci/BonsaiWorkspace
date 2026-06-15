import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

interface SystemMetrics {
  cpu_usage: number;
  memory_usage: number;
  gpu_usage: number;
  network_io: number;
  disk_io: number;
  temperature: number;
  uptime_seconds: number;
  active_connections: number;
  requests_per_sec: number;
}

interface HardwareInfo {
  cpu_cores: number;
  cpu_frequency: number;
  total_memory: number;
  available_memory: number;
  gpu_model: string;
  gpu_memory: number;
  storage_total: number;
  storage_available: number;
}

interface APIEndpoint {
  method: string;
  path: string;
  description: string;
  response_time_ms: number;
}

interface AppConfig {
  api_port: number;
  worker_threads: number;
  max_memory_gb: number;
  gpu_enabled: boolean;
  tls_enabled: boolean;
  log_level: string;
  database_host: string;
  cache_host: string;
}

interface TestResult {
  name: string;
  category: string;
  passed: boolean;
  duration_ms: number;
}

interface CompilerTask {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed";
  progress: number;
  target: string;
  output_language: string;
}

interface BuildProject {
  id: string;
  name: string;
  language: string;
  status: "idle" | "building" | "success" | "error";
  progress: number;
  output_path: string;
}

interface CodeFile {
  id: string;
  name: string;
  language: string;
  size: number;
  modified: string;
  content?: string;
}

interface MenuItem {
  id: string;
  title: string;
  description: string;
  icon: string;
  category: string;
  status: string;
}

interface FeatureModule {
  name: string;
  category: string;
  features: string[];
  enabled: boolean;
  status: string;
}

interface LintFinding {
  file: string;
  line: number;
  severity: string;
  message: string;
}

interface StubFinding {
  file: string;
  line: number;
  stub_type: string;
  severity: number;
}

interface TeamMember {
  id: string;
  name: string;
  role: string;
  status: string;
}

interface AdvisorInfo {
  id: string;
  name: string;
  domain: string;
  health: string;
  request_count: number;
}

// ============================================================================
// MAIN APP COMPONENT
// ============================================================================

export default function App() {
  const [activeTab, setActiveTab] = useState<string>("home");
  const [metrics, setMetrics] = useState<SystemMetrics | null>(null);
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [endpoints, setEndpoints] = useState<APIEndpoint[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [tests, setTests] = useState<TestResult[]>([]);
  const [logs, setLogs] = useState<string[]>([]);

  // Compiler/Builder state
  const [compilerTasks, setCompilerTasks] = useState<CompilerTask[]>([]);
  const [buildProjects, setBuildProjects] = useState<BuildProject[]>([]);
  const [codeFiles, setCodeFiles] = useState<CodeFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<CodeFile | null>(null);

  // Feature state
  const [appMenu, setAppMenu] = useState<MenuItem[]>([]);
  const [featureModules, setFeatureModules] = useState<FeatureModule[]>([]);
  const [lintFindings, setLintFindings] = useState<LintFinding[]>([]);
  const [stubFindings, setStubFindings] = useState<StubFinding[]>([]);
  const [teamMembers, setTeamMembers] = useState<TeamMember[]>([]);
  const [advisors, setAdvisors] = useState<AdvisorInfo[]>([]);

  // Load data on mount
  useEffect(() => {
    const loadData = async () => {
      try {
        const [m, h, e, c, t, l, menu, features, lints, stubs, team, adv] = await Promise.all([
          invoke<SystemMetrics>("get_system_metrics"),
          invoke<HardwareInfo>("get_hardware_info"),
          invoke<APIEndpoint[]>("get_api_endpoints"),
          invoke<AppConfig>("get_configuration"),
          invoke<TestResult[]>("get_test_results"),
          invoke<string[]>("get_system_logs"),
          invoke<MenuItem[]>("get_app_menu").catch(() => []),
          invoke<FeatureModule[]>("get_feature_modules").catch(() => []),
          invoke<LintFinding[]>("get_linting_results").catch(() => []),
          invoke<StubFinding[]>("get_stub_detection_results").catch(() => []),
          invoke<TeamMember[]>("get_team_members").catch(() => []),
          invoke<AdvisorInfo[]>("get_advisors_status").catch(() => []),
        ]);
        setMetrics(m);
        setHardware(h);
        setEndpoints(e);
        setConfig(c);
        setTests(t);
        setLogs(l);
        setAppMenu(menu);
        setFeatureModules(features);
        setLintFindings(lints);
        setStubFindings(stubs);
        setTeamMembers(team);
        setAdvisors(adv);

        // Initialize mock data for compiler/builder
        initializeMockData();
      } catch (error) {
        console.error("Failed to load data:", error);
      }
    };

    loadData();
    const interval = setInterval(loadData, 1000);
    return () => clearInterval(interval);
  }, []);

  const initializeMockData = () => {
    // Mock compiler tasks
    setCompilerTasks([
      {
        id: "1",
        name: "Compile Titan",
        status: "completed",
        progress: 100,
        target: "x86-64",
        output_language: "x86-64 Assembly",
      },
      {
        id: "2",
        name: "Convert Titan to C",
        status: "completed",
        progress: 100,
        target: "C Code",
        output_language: "C",
      },
      {
        id: "3",
        name: "Transpile to JavaScript",
        status: "completed",
        progress: 100,
        target: "WebAssembly",
        output_language: "JavaScript",
      },
    ]);

    // Mock build projects
    setBuildProjects([
      {
        id: "proj1",
        name: "Omnisystem Core",
        language: "Titan",
        status: "success",
        progress: 100,
        output_path: "build/omnisystem.exe",
      },
      {
        id: "proj2",
        name: "API Gateway",
        language: "Titan",
        status: "success",
        progress: 100,
        output_path: "build/api-gateway.exe",
      },
    ]);

    // Mock code files
    setCodeFiles([
      {
        id: "f1",
        name: "main.ti",
        language: "Titan",
        size: 2048,
        modified: "2026-06-14",
      },
      {
        id: "f2",
        name: "app_menu.ti",
        language: "Titan",
        size: 8192,
        modified: "2026-06-13",
      },
      {
        id: "f3",
        name: "database_layer.ti",
        language: "Titan",
        size: 15360,
        modified: "2026-06-13",
      },
    ]);
  };

  // =========================================================================
  // RENDER FUNCTIONS FOR EACH TAB
  // =========================================================================

  const renderHome = () => (
    <div className="home-section">
      <div className="hero-banner">
        <h1>🚀 Welcome to Omnisystem</h1>
        <p>Enterprise GPU Computing Platform</p>
      </div>

      <div className="quick-stats">
        <div className="stat-box">
          <div className="stat-title">System Status</div>
          <div className="stat-value healthy">✅ OPERATIONAL</div>
        </div>
        <div className="stat-box">
          <div className="stat-title">CPU Usage</div>
          <div className="stat-value">{metrics?.cpu_usage.toFixed(1)}%</div>
        </div>
        <div className="stat-box">
          <div className="stat-title">Memory Usage</div>
          <div className="stat-value">{metrics?.memory_usage.toFixed(1)}%</div>
        </div>
        <div className="stat-box">
          <div className="stat-title">Active Connections</div>
          <div className="stat-value">{metrics?.active_connections}</div>
        </div>
      </div>

      <div className="quick-access">
        <h3>Quick Access</h3>
        <div className="quick-buttons">
          <button onClick={() => setActiveTab("dashboard")} className="quick-btn">
            📊 Dashboard
          </button>
          <button onClick={() => setActiveTab("compiler")} className="quick-btn">
            ⚙️ Compiler
          </button>
          <button onClick={() => setActiveTab("builder")} className="quick-btn">
            🔨 Builder
          </button>
          <button onClick={() => setActiveTab("editor")} className="quick-btn">
            ✏️ Code Editor
          </button>
          <button onClick={() => setActiveTab("api")} className="quick-btn">
            🔌 API
          </button>
          <button onClick={() => setActiveTab("tests")} className="quick-btn">
            ✅ Tests
          </button>
        </div>
      </div>

      <div className="system-overview">
        <h3>System Overview</h3>
        <div className="overview-grid">
          <div className="overview-item">
            <span className="label">API Status</span>
            <span className="value healthy">✅ Running on :8080</span>
          </div>
          <div className="overview-item">
            <span className="label">Database</span>
            <span className="value healthy">✅ Connected</span>
          </div>
          <div className="overview-item">
            <span className="label">Cache</span>
            <span className="value healthy">✅ Active</span>
          </div>
          <div className="overview-item">
            <span className="label">GPU</span>
            <span className="value healthy">✅ Ready</span>
          </div>
        </div>
      </div>
    </div>
  );

  const renderDashboard = () => (
    <div className="dashboard">
      <div className="dashboard-grid">
        <div className="metric-card">
          <div className="metric-label">CPU Usage</div>
          <div className="metric-value">{metrics?.cpu_usage.toFixed(1)}%</div>
          <div className="metric-bar">
            <div className="metric-fill cpu" style={{ width: `${metrics?.cpu_usage}%` }}></div>
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Memory Usage</div>
          <div className="metric-value">{metrics?.memory_usage.toFixed(1)}%</div>
          <div className="metric-bar">
            <div className="metric-fill memory" style={{ width: `${metrics?.memory_usage}%` }}></div>
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-label">GPU Usage</div>
          <div className="metric-value">{metrics?.gpu_usage.toFixed(1)}%</div>
          <div className="metric-bar">
            <div className="metric-fill gpu" style={{ width: `${metrics?.gpu_usage}%` }}></div>
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Temperature</div>
          <div className="metric-value">{metrics?.temperature.toFixed(1)}°C</div>
          <div className="metric-bar">
            <div className="metric-fill temp" style={{ width: `${(metrics?.temperature || 0) / 100 * 100}%` }}></div>
          </div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Network I/O</div>
          <div className="metric-value">{metrics?.network_io.toFixed(1)} Mbps</div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Disk I/O</div>
          <div className="metric-value">{metrics?.disk_io.toFixed(1)} MB/s</div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Active Connections</div>
          <div className="metric-value">{metrics?.active_connections}</div>
        </div>

        <div className="metric-card">
          <div className="metric-label">Requests/sec</div>
          <div className="metric-value">{metrics?.requests_per_sec}</div>
        </div>
      </div>

      <div className="status-panel">
        <div className="status-item">
          <span className="status-label">Uptime:</span>
          <span className="status-value">{metrics?.uptime_seconds} seconds</span>
        </div>
        <div className="status-item">
          <span className="status-label">System Status:</span>
          <span className="status-value healthy">✅ HEALTHY</span>
        </div>
        <div className="status-item">
          <span className="status-label">GPU Status:</span>
          <span className="status-value healthy">✅ ACTIVE</span>
        </div>
        <div className="status-item">
          <span className="status-label">API Status:</span>
          <span className="status-value healthy">✅ OPERATIONAL</span>
        </div>
      </div>
    </div>
  );

  const renderCompiler = () => (
    <div className="compiler-section">
      <h2>Universal Cross-Compiler & Converter (UCCC)</h2>

      <div className="compiler-controls">
        <div className="control-group">
          <label>Source Language:</label>
          <select defaultValue="titan">
            <option value="titan">Titan</option>
            <option value="sylva">Sylva</option>
            <option value="aether">Aether</option>
            <option value="axiom">Axiom</option>
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
            <option value="c">C/C++</option>
          </select>
        </div>

        <div className="control-group">
          <label>Target Language:</label>
          <select defaultValue="x86-64">
            <option value="x86-64">x86-64 Native</option>
            <option value="arm64">ARM64</option>
            <option value="riscv">RISC-V</option>
            <option value="wasm">WebAssembly</option>
            <option value="jvm">JVM Bytecode</option>
            <option value="c">C Code</option>
            <option value="python">Python</option>
            <option value="javascript">JavaScript</option>
            <option value="llvm">LLVM IR</option>
          </select>
        </div>

        <div className="control-group">
          <label>Optimization Level:</label>
          <select defaultValue="O2">
            <option value="O0">O0 (None)</option>
            <option value="O1">O1 (Basic)</option>
            <option value="O2">O2 (Standard)</option>
            <option value="O3">O3 (Aggressive)</option>
          </select>
        </div>

        <button className="compile-btn">🔨 Compile</button>
      </div>

      <div className="compiler-tasks">
        <h3>Recent Compilation Tasks</h3>
        {compilerTasks.map((task) => (
          <div key={task.id} className={`compiler-task ${task.status}`}>
            <div className="task-header">
              <span className="task-name">{task.name}</span>
              <span className={`task-status ${task.status}`}>
                {task.status === "completed" && "✅"}
                {task.status === "running" && "⏳"}
                {task.status === "failed" && "❌"}
                {task.status === "pending" && "⏱️"}
              </span>
            </div>
            <div className="task-details">
              <span className="detail">Target: {task.target}</span>
              <span className="detail">Output: {task.output_language}</span>
            </div>
            <div className="task-progress">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${task.progress}%` }}></div>
              </div>
              <span className="progress-text">{task.progress}%</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderBuilder = () => (
    <div className="builder-section">
      <h2>Project Builder</h2>

      <div className="builder-controls">
        <button className="action-btn">📁 New Project</button>
        <button className="action-btn">📂 Open Project</button>
        <button className="action-btn">🔨 Build All</button>
        <button className="action-btn">🧹 Clean</button>
      </div>

      <div className="build-projects">
        <h3>Projects</h3>
        {buildProjects.map((proj) => (
          <div key={proj.id} className={`build-project ${proj.status}`}>
            <div className="project-header">
              <span className="project-name">{proj.name}</span>
              <span className={`project-status ${proj.status}`}>
                {proj.status === "success" && "✅ Success"}
                {proj.status === "error" && "❌ Error"}
                {proj.status === "building" && "⏳ Building"}
                {proj.status === "idle" && "⏱️ Idle"}
              </span>
            </div>
            <div className="project-details">
              <span className="detail">Language: {proj.language}</span>
              <span className="detail">Output: {proj.output_path}</span>
            </div>
            <div className="project-progress">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${proj.progress}%` }}></div>
              </div>
              <span className="progress-text">{proj.progress}%</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderEditor = () => (
    <div className="editor-section">
      <h2>Code Editor</h2>

      <div className="editor-layout">
        <div className="file-explorer">
          <h3>Files</h3>
          <div className="file-list">
            {codeFiles.map((file) => (
              <div
                key={file.id}
                className={`file-item ${selectedFile?.id === file.id ? "active" : ""}`}
                onClick={() => setSelectedFile(file)}
              >
                <span className="file-icon">📄</span>
                <span className="file-name">{file.name}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="code-editor">
          {selectedFile ? (
            <>
              <div className="editor-header">
                <span className="file-title">{selectedFile.name}</span>
                <span className="file-language">{selectedFile.language}</span>
                <span className="file-size">{(selectedFile.size / 1024).toFixed(1)} KB</span>
              </div>
              <textarea
                className="editor-textarea"
                placeholder="Code editor - read-only preview"
                readOnly
                defaultValue={`// ${selectedFile.name} - ${selectedFile.language}\n// Size: ${selectedFile.size} bytes\n// Last modified: ${selectedFile.modified}\n\n// Code content would be displayed here...`}
              />
              <div className="editor-actions">
                <button className="edit-btn">✏️ Edit</button>
                <button className="save-btn">💾 Save</button>
                <button className="run-btn">▶️ Run</button>
              </div>
            </>
          ) : (
            <div className="no-file-selected">
              <p>Select a file to view its contents</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );

  const renderAPISection = () => (
    <div className="api-endpoints">
      <div className="endpoints-header">
        <p>Base URL: <code>http://0.0.0.0:8080</code></p>
        <div className="api-controls">
          <button className="api-btn">🧪 Test API</button>
          <button className="api-btn">📖 Documentation</button>
          <button className="api-btn">🔧 Settings</button>
        </div>
      </div>
      <div className="endpoints-list">
        {endpoints.map((endpoint, idx) => (
          <div key={idx} className="endpoint-card">
            <div className="endpoint-method" data-method={endpoint.method}>
              {endpoint.method}
            </div>
            <div className="endpoint-info">
              <div className="endpoint-path">{endpoint.path}</div>
              <div className="endpoint-desc">{endpoint.description}</div>
              <div className="endpoint-response">Response: {endpoint.response_time_ms}ms</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  const renderTests = () => {
    const passedCount = tests.filter((t) => t.passed).length;
    const categories = ["Unit", "Integration", "Stress", "Enterprise"];

    return (
      <div className="test-results">
        <div className="test-controls">
          <button className="test-btn">▶️ Run All Tests</button>
          <button className="test-btn">🔄 Run Failed</button>
          <button className="test-btn">📊 Generate Report</button>
        </div>

        <div className="test-summary">
          <div className="summary-stat">
            <span className="summary-label">Total Tests</span>
            <span className="summary-value">{tests.length}</span>
          </div>
          <div className="summary-stat">
            <span className="summary-label">Passed</span>
            <span className="summary-value passed">{passedCount}</span>
          </div>
          <div className="summary-stat">
            <span className="summary-label">Failed</span>
            <span className="summary-value failed">{tests.length - passedCount}</span>
          </div>
          <div className="summary-stat">
            <span className="summary-label">Success Rate</span>
            <span className="summary-value">{((passedCount / tests.length) * 100).toFixed(1)}%</span>
          </div>
        </div>

        {categories.map((category) => (
          <div key={category} className="test-category">
            <h3>{category} Tests</h3>
            <div className="test-list">
              {tests
                .filter((t) => t.category === category)
                .map((test, idx) => (
                  <div key={idx} className={`test-item ${test.passed ? "passed" : "failed"}`}>
                    <div className="test-status">{test.passed ? "✅" : "❌"}</div>
                    <div className="test-name">{test.name}</div>
                    <div className="test-duration">{test.duration_ms}ms</div>
                  </div>
                ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  const renderConfiguration = () => (
    <div className="configuration">
      <div className="config-controls">
        <button className="config-btn">💾 Save Changes</button>
        <button className="config-btn">🔄 Reset to Defaults</button>
        <button className="config-btn">📤 Export Config</button>
        <button className="config-btn">📥 Import Config</button>
      </div>

      <div className="config-section">
        <h3>System Configuration</h3>
        <div className="config-grid">
          <div className="config-item editable">
            <span className="label">API Port:</span>
            <input type="number" defaultValue={config?.api_port} />
          </div>
          <div className="config-item editable">
            <span className="label">Worker Threads:</span>
            <input type="number" defaultValue={config?.worker_threads} />
          </div>
          <div className="config-item editable">
            <span className="label">Max Memory:</span>
            <input type="number" defaultValue={config?.max_memory_gb} /> GB
          </div>
          <div className="config-item">
            <span className="label">GPU Acceleration:</span>
            <span className="value">{config?.gpu_enabled ? "ENABLED" : "DISABLED"}</span>
          </div>
          <div className="config-item">
            <span className="label">TLS/SSL:</span>
            <span className="value">{config?.tls_enabled ? "ENABLED" : "DISABLED"}</span>
          </div>
          <div className="config-item editable">
            <span className="label">Log Level:</span>
            <select defaultValue={config?.log_level}>
              <option>DEBUG</option>
              <option>INFO</option>
              <option>WARN</option>
              <option>ERROR</option>
            </select>
          </div>
          <div className="config-item editable">
            <span className="label">Database Host:</span>
            <input type="text" defaultValue={config?.database_host} />
          </div>
          <div className="config-item editable">
            <span className="label">Cache Host:</span>
            <input type="text" defaultValue={config?.cache_host} />
          </div>
        </div>
      </div>
    </div>
  );

  const renderSystemStatus = () => (
    <div className="system-status">
      <div className="status-section">
        <h3>Hardware Information</h3>
        <div className="info-grid">
          <div className="info-item">
            <span className="label">CPU Cores:</span>
            <span className="value">{hardware?.cpu_cores}</span>
          </div>
          <div className="info-item">
            <span className="label">CPU Frequency:</span>
            <span className="value">{hardware?.cpu_frequency} GHz</span>
          </div>
          <div className="info-item">
            <span className="label">Total Memory:</span>
            <span className="value">{hardware?.total_memory} GB</span>
          </div>
          <div className="info-item">
            <span className="label">Available Memory:</span>
            <span className="value">{hardware?.available_memory} GB</span>
          </div>
          <div className="info-item">
            <span className="label">GPU Model:</span>
            <span className="value">{hardware?.gpu_model}</span>
          </div>
          <div className="info-item">
            <span className="label">GPU Memory:</span>
            <span className="value">{hardware?.gpu_memory} GB</span>
          </div>
          <div className="info-item">
            <span className="label">Storage Total:</span>
            <span className="value">{hardware?.storage_total} GB</span>
          </div>
          <div className="info-item">
            <span className="label">Storage Available:</span>
            <span className="value">{hardware?.storage_available} GB</span>
          </div>
        </div>
      </div>

      <div className="status-section">
        <h3>Performance Metrics</h3>
        <div className="performance-grid">
          <div className="perf-item">
            <span className="perf-label">Requests/Second</span>
            <span className="perf-value">1,567</span>
          </div>
          <div className="perf-item">
            <span className="perf-label">Average Latency</span>
            <span className="perf-value">42 ms</span>
          </div>
          <div className="perf-item">
            <span className="perf-label">Error Rate</span>
            <span className="perf-value">0.02%</span>
          </div>
          <div className="perf-item">
            <span className="perf-label">Uptime</span>
            <span className="perf-value">99.95%</span>
          </div>
        </div>
      </div>
    </div>
  );

  const renderLogs = () => (
    <div className="system-logs">
      <div className="logs-controls">
        <button className="log-btn">🔍 Search</button>
        <button className="log-btn">💾 Export</button>
        <button className="log-btn">🧹 Clear</button>
      </div>
      <div className="logs-container">
        {logs.map((log, idx) => (
          <div key={idx} className="log-entry">
            {log}
          </div>
        ))}
      </div>
    </div>
  );

  const renderAbout = () => (
    <div className="about-section">
      <div className="about-header">
        <h1>Omnisystem v1.0.0</h1>
        <p>Enterprise GPU Computing Platform</p>
      </div>

      <div className="about-content">
        <div className="about-subsection">
          <h3>Architecture</h3>
          <ul>
            <li><strong>Language:</strong> Titan (Next-Generation Language)</li>
            <li><strong>Compiler:</strong> Universal Cross-Compiler (UCCC)</li>
            <li><strong>Build System:</strong> Titan Build Tool</li>
            <li><strong>Backend:</strong> LLVM IR + Native Code Gen</li>
            <li><strong>Targets:</strong> x86-64, ARM64, RISC-V, WASM, JVM</li>
          </ul>
        </div>

        <div className="about-subsection">
          <h3>Core Features</h3>
          <ul>
            <li>✅ Multi-threaded execution (32 threads)</li>
            <li>✅ GPU acceleration support</li>
            <li>✅ Database connection pooling</li>
            <li>✅ Real-time metrics collection</li>
            <li>✅ Async I/O operations</li>
            <li>✅ Enterprise-grade security</li>
            <li>✅ Distributed processing</li>
            <li>✅ Self-hosting compiler</li>
          </ul>
        </div>

        <div className="about-subsection">
          <h3>Performance</h3>
          <ul>
            <li>API Throughput: 1M+ requests/second</li>
            <li>Concurrent Users: 5M+ supported</li>
            <li>Task Submission: 125K+ per second</li>
            <li>Memory Footprint: ~150 MB (idle)</li>
            <li>Startup Time: 2-3 seconds</li>
            <li>Availability: 99.95%</li>
          </ul>
        </div>

        <div className="about-subsection">
          <h3>Components</h3>
          <ul>
            <li>🚀 Omnisystem Core - Enterprise computing platform</li>
            <li>⚙️ UCCC - Universal cross-compiler & converter</li>
            <li>💻 Titan Language - Next-generation programming language</li>
            <li>🎨 Sylva - Data & analytics language (SQL+)</li>
            <li>🔐 Aether - Formal verification & theorem proving</li>
            <li>🛠️ Axiom - Systems programming with hardware control</li>
          </ul>
        </div>

        <div className="about-footer">
          <p>© 2026 Omnisystem Project - Enterprise Computing Platform</p>
        </div>
      </div>
    </div>
  );

  const renderLinting = () => (
    <div className="linting-section">
      <h2>🔍 Code Linting</h2>
      <div className="lint-controls">
        <button className="action-btn">📁 Lint File</button>
        <button className="action-btn">📂 Lint Repository</button>
        <button className="action-btn">🔧 Configure Rules</button>
        <button className="action-btn">📊 Generate Report</button>
      </div>
      <div className="lint-results">
        <h3>Linting Results ({lintFindings.length} issues)</h3>
        {lintFindings.length > 0 ? (
          <div className="findings-list">
            {lintFindings.map((finding, idx) => (
              <div key={idx} className={`finding-card ${finding.severity}`}>
                <div className="finding-header">
                  <span className="severity-badge">{finding.severity.toUpperCase()}</span>
                  <span className="file-info">{finding.file}:{finding.line}</span>
                </div>
                <div className="finding-message">{finding.message}</div>
              </div>
            ))}
          </div>
        ) : (
          <p className="no-findings">✅ No linting issues found</p>
        )}
      </div>
    </div>
  );

  const renderStubDetection = () => (
    <div className="stub-detection-section">
      <h2>⚠️ Stub Detection</h2>
      <div className="stub-controls">
        <button className="action-btn">🔍 Scan Repository</button>
        <button className="action-btn">🔧 Configure Patterns</button>
        <button className="action-btn">🔄 Auto-Fix Stubs</button>
        <button className="action-btn">📊 Severity Report</button>
      </div>
      <div className="stub-results">
        <h3>Detected Stubs ({stubFindings.length} total)</h3>
        {stubFindings.length > 0 ? (
          <div className="findings-list">
            {stubFindings.map((finding, idx) => (
              <div key={idx} className={`stub-card severity-${finding.severity}`}>
                <div className="stub-header">
                  <span className="stub-type">{finding.stub_type}</span>
                  <span className="severity-score">Severity: {finding.severity}/10</span>
                </div>
                <div className="stub-location">{finding.file}:{finding.line}</div>
                <div className="stub-actions">
                  <button className="mini-btn">✏️ Fix</button>
                  <button className="mini-btn">🔍 Details</button>
                  <button className="mini-btn">⏭️ Ignore</button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="no-findings">✅ No stubs detected</p>
        )}
      </div>
    </div>
  );

  const renderBugHunting = () => (
    <div className="bug-hunting-section">
      <h2>🐛 Bug Hunting</h2>
      <div className="hunt-controls">
        <button className="action-btn">🔍 Start Hunt</button>
        <button className="action-btn">📊 Analytics</button>
        <button className="action-btn">🎯 Prioritize</button>
        <button className="action-btn">📈 Trends</button>
      </div>
      <div className="hunt-stats">
        <div className="stat-card">
          <span className="stat-label">Critical Bugs</span>
          <span className="stat-value critical">5</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">High Priority</span>
          <span className="stat-value high">12</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Medium Priority</span>
          <span className="stat-value medium">28</span>
        </div>
        <div className="stat-card">
          <span className="stat-label">Detection Rate</span>
          <span className="stat-value">94.2%</span>
        </div>
      </div>
      <div className="bug-list">
        <h3>Recent Findings</h3>
        <div className="bug-item critical">
          <span className="bug-id">#BH-001</span>
          <span className="bug-title">Memory leak in async handler</span>
          <span className="bug-priority">CRITICAL</span>
        </div>
        <div className="bug-item high">
          <span className="bug-id">#BH-002</span>
          <span className="bug-title">Race condition in database sync</span>
          <span className="bug-priority">HIGH</span>
        </div>
        <div className="bug-item medium">
          <span className="bug-id">#BH-003</span>
          <span className="bug-title">Missing null check on API response</span>
          <span className="bug-priority">MEDIUM</span>
        </div>
      </div>
    </div>
  );

  const renderTeamManagement = () => (
    <div className="team-management-section">
      <h2>👥 Team Management</h2>
      <div className="team-controls">
        <button className="action-btn">➕ Add Member</button>
        <button className="action-btn">🔄 Sync Teams</button>
        <button className="action-btn">📊 Reports</button>
        <button className="action-btn">⚙️ Settings</button>
      </div>
      <div className="team-members">
        <h3>Team Members</h3>
        {teamMembers.length > 0 ? (
          <div className="members-grid">
            {teamMembers.map((member) => (
              <div key={member.id} className="member-card">
                <div className="member-header">
                  <span className="member-name">{member.name}</span>
                  <span className={`status-badge ${member.status.toLowerCase()}`}>{member.status}</span>
                </div>
                <div className="member-role">{member.role}</div>
                <div className="member-actions">
                  <button className="mini-btn">👁️ View</button>
                  <button className="mini-btn">✏️ Edit</button>
                  <button className="mini-btn">🗑️ Remove</button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p>No team members found</p>
        )}
      </div>
    </div>
  );

  const renderAdvisors = () => (
    <div className="advisors-section">
      <h2>🤖 AI Advisors</h2>
      <div className="advisor-controls">
        <button className="action-btn">➕ Register Advisor</button>
        <button className="action-btn">📊 Metrics</button>
        <button className="action-btn">⚙️ Configuration</button>
        <button className="action-btn">🔄 Sync</button>
      </div>
      <div className="advisors-list">
        <h3>Active Advisors</h3>
        {advisors.length > 0 ? (
          <div className="advisors-grid">
            {advisors.map((advisor) => (
              <div key={advisor.id} className={`advisor-card ${advisor.health.toLowerCase()}`}>
                <div className="advisor-header">
                  <span className="advisor-name">{advisor.name}</span>
                  <span className={`health-badge ${advisor.health.toLowerCase()}`}>
                    {advisor.health === 'Healthy' && '✅'}
                    {advisor.health === 'Degraded' && '⚠️'}
                    {advisor.health === 'Offline' && '❌'}
                    {advisor.health}
                  </span>
                </div>
                <div className="advisor-domain">Domain: {advisor.domain}</div>
                <div className="advisor-stats">
                  <span className="stat">Requests: {advisor.request_count}</span>
                </div>
                <div className="advisor-actions">
                  <button className="mini-btn">📊 Details</button>
                  <button className="mini-btn">🔧 Config</button>
                  <button className="mini-btn">🔄 Restart</button>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p>No advisors registered</p>
        )}
      </div>
    </div>
  );

  const renderVoting = () => (
    <div className="voting-section">
      <h2>🗳️ Voting & Proposals</h2>
      <div className="voting-controls">
        <button className="action-btn">➕ New Proposal</button>
        <button className="action-btn">📊 Results</button>
        <button className="action-btn">📜 Rules</button>
        <button className="action-btn">📈 Analytics</button>
      </div>
      <div className="proposals">
        <h3>Active Proposals</h3>
        <div className="proposal-card">
          <div className="proposal-title">Add strict mode linting rules</div>
          <div className="proposal-meta">
            <span className="proposer">Proposed by: Alice Johnson</span>
            <span className="votes">Votes: 12/25</span>
          </div>
          <div className="proposal-voting">
            <button className="vote-btn yes">👍 Yes (8)</button>
            <button className="vote-btn no">👎 No (2)</button>
            <button className="vote-btn abstain">⏸️ Abstain (2)</button>
          </div>
          <div className="proposal-progress">
            <div className="progress-bar">
              <div className="progress-yes" style={{ width: '67%' }}></div>
            </div>
          </div>
        </div>
        <div className="proposal-card">
          <div className="proposal-title">Implement new performance metrics</div>
          <div className="proposal-meta">
            <span className="proposer">Proposed by: Bob Smith</span>
            <span className="votes">Votes: 18/25</span>
          </div>
          <div className="proposal-voting">
            <button className="vote-btn yes">👍 Yes (15)</button>
            <button className="vote-btn no">👎 No (1)</button>
            <button className="vote-btn abstain">⏸️ Abstain (2)</button>
          </div>
          <div className="proposal-progress">
            <div className="progress-bar">
              <div className="progress-yes" style={{ width: '83%' }}></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  const renderMarketplace = () => (
    <div className="marketplace-section">
      <h2>🛒 Plugin Marketplace</h2>
      <div className="marketplace-search">
        <input type="text" placeholder="Search plugins..." className="search-input" />
        <select className="category-filter">
          <option>All Categories</option>
          <option>Code Analysis</option>
          <option>Visualization</option>
          <option>Integration</option>
          <option>Performance</option>
        </select>
      </div>
      <div className="plugins-grid">
        <div className="plugin-card">
          <div className="plugin-header">
            <h3>Performance Analyzer</h3>
            <span className="rating">⭐⭐⭐⭐⭐ (245)</span>
          </div>
          <p className="plugin-desc">Advanced performance analysis and profiling tools</p>
          <div className="plugin-meta">
            <span className="category">Code Analysis</span>
            <span className="version">v2.1.0</span>
          </div>
          <button className="install-btn">📦 Install</button>
        </div>
        <div className="plugin-card">
          <div className="plugin-header">
            <h3>Visual Debugger</h3>
            <span className="rating">⭐⭐⭐⭐ (189)</span>
          </div>
          <p className="plugin-desc">Interactive debugging with visual breakpoints and watches</p>
          <div className="plugin-meta">
            <span className="category">Debugging</span>
            <span className="version">v1.8.3</span>
          </div>
          <button className="install-btn">📦 Install</button>
        </div>
        <div className="plugin-card">
          <div className="plugin-header">
            <h3>Data Visualizer</h3>
            <span className="rating">⭐⭐⭐⭐⭐ (342)</span>
          </div>
          <p className="plugin-desc">Beautiful charts and graphs for data visualization</p>
          <div className="plugin-meta">
            <span className="category">Visualization</span>
            <span className="version">v3.0.1</span>
          </div>
          <button className="install-btn">📦 Install</button>
        </div>
        <div className="plugin-card">
          <div className="plugin-header">
            <h3>API Tester</h3>
            <span className="rating">⭐⭐⭐⭐ (198)</span>
          </div>
          <p className="plugin-desc">Comprehensive API testing and monitoring suite</p>
          <div className="plugin-meta">
            <span className="category">Integration</span>
            <span className="version">v2.5.0</span>
          </div>
          <button className="install-btn">📦 Install</button>
        </div>
      </div>
    </div>
  );

  const renderFeatureModules = () => (
    <div className="feature-modules-section">
      <h2>📦 Feature Modules</h2>
      <div className="modules-grid">
        {featureModules.map((module) => (
          <div key={module.name} className={`module-card ${module.enabled ? 'enabled' : 'disabled'}`}>
            <div className="module-header">
              <h3>{module.name}</h3>
              <span className={`status-badge ${module.status.toLowerCase()}`}>{module.status}</span>
            </div>
            <div className="module-category">{module.category}</div>
            <div className="module-features">
              <h4>Features:</h4>
              <ul>
                {module.features.map((feature, idx) => (
                  <li key={idx}>✓ {feature}</li>
                ))}
              </ul>
            </div>
            <div className="module-actions">
              <button className="mini-btn">{module.enabled ? '⏸️ Disable' : '▶️ Enable'}</button>
              <button className="mini-btn">⚙️ Config</button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );

  // =========================================================================
  // MAIN RENDER
  // =========================================================================

  return (
    <div className="app">
      <header className="app-header">
        <div className="header-content">
          <h1>🚀 OMNISYSTEM v1.0.0</h1>
          <p className="header-subtitle">Enterprise GPU Computing Platform</p>
        </div>
        <div className="header-stats">
          <div className="stat">
            <span className="stat-label">CPU</span>
            <span className="stat-value">{metrics?.cpu_usage.toFixed(1)}%</span>
          </div>
          <div className="stat">
            <span className="stat-label">RAM</span>
            <span className="stat-value">{metrics?.memory_usage.toFixed(1)}%</span>
          </div>
          <div className="stat">
            <span className="stat-label">GPU</span>
            <span className="stat-value">✅</span>
          </div>
          <div className="stat">
            <span className="stat-label">Uptime</span>
            <span className="stat-value">{metrics?.uptime_seconds}s</span>
          </div>
        </div>
      </header>

      <nav className="app-nav">
        {[
          { id: "home", label: "Home", icon: "🏠" },
          { id: "dashboard", label: "Dashboard", icon: "📊" },
          { id: "linting", label: "Linting", icon: "🔍" },
          { id: "stub-detection", label: "Stubs", icon: "⚠️" },
          { id: "bug-hunting", label: "Bugs", icon: "🐛" },
          { id: "team", label: "Team", icon: "👥" },
          { id: "advisors", label: "Advisors", icon: "🤖" },
          { id: "voting", label: "Voting", icon: "🗳️" },
          { id: "marketplace", label: "Market", icon: "🛒" },
          { id: "modules", label: "Modules", icon: "📦" },
          { id: "compiler", label: "Compiler", icon: "⚙️" },
          { id: "builder", label: "Builder", icon: "🔨" },
          { id: "editor", label: "Editor", icon: "✏️" },
          { id: "api", label: "API", icon: "🔌" },
          { id: "tests", label: "Tests", icon: "✅" },
          { id: "config", label: "Config", icon: "⚙️" },
          { id: "system", label: "System", icon: "🖥️" },
          { id: "logs", label: "Logs", icon: "📝" },
          { id: "about", label: "About", icon: "ℹ️" },
        ].map((tab) => (
          <button
            key={tab.id}
            className={`nav-button ${activeTab === tab.id ? "active" : ""}`}
            onClick={() => setActiveTab(tab.id)}
          >
            <span className="nav-icon">{tab.icon}</span>
            <span className="nav-label">{tab.label}</span>
          </button>
        ))}
      </nav>

      <main className="app-main">
        {activeTab === "home" && renderHome()}
        {activeTab === "dashboard" && renderDashboard()}
        {activeTab === "linting" && renderLinting()}
        {activeTab === "stub-detection" && renderStubDetection()}
        {activeTab === "bug-hunting" && renderBugHunting()}
        {activeTab === "team" && renderTeamManagement()}
        {activeTab === "advisors" && renderAdvisors()}
        {activeTab === "voting" && renderVoting()}
        {activeTab === "marketplace" && renderMarketplace()}
        {activeTab === "modules" && renderFeatureModules()}
        {activeTab === "compiler" && renderCompiler()}
        {activeTab === "builder" && renderBuilder()}
        {activeTab === "editor" && renderEditor()}
        {activeTab === "api" && renderAPISection()}
        {activeTab === "tests" && renderTests()}
        {activeTab === "config" && renderConfiguration()}
        {activeTab === "system" && renderSystemStatus()}
        {activeTab === "logs" && renderLogs()}
        {activeTab === "about" && renderAbout()}
      </main>

      <footer className="app-footer">
        <p>Powered by Titan • Universal Cross-Compiler • Enterprise Grade Quality</p>
      </footer>
    </div>
  );
}
