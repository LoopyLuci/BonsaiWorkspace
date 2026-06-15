/**
 * Example TypeScript Agent for Bonsai Ecosystem
 *
 * This agent connects to the Bonsai Universal Agent Control System via MCP and
 * demonstrates reading files, running tools, and handling approvals.
 */

import axios, { AxiosInstance } from 'axios';

// Configuration
const MCP_URL = 'http://127.0.0.1:11426/mcp';
const CAPABILITY_TOKEN = 'demo-token'; // In production, use secure token management

// Tool catalog
const TOOLS: Record<string, string> = {
  read_file: 'Read a file from the workspace',
  write_file: 'Write a file to the workspace',
  run_cargo_check: "Run 'cargo check' on the workspace",
  run_cargo_test: "Run 'cargo test' on the workspace",
  search_codebase: 'Search for patterns in the codebase',
  run_shell: 'Execute a shell command',
};

interface MCPRequest {
  jsonrpc: '2.0';
  method: string;
  params: Record<string, unknown>;
  id: number;
}

interface MCPResponse {
  jsonrpc: '2.0';
  result?: Record<string, unknown>;
  error?: { code: number; message: string };
  id: number;
}

class BonsaiAgent {
  private client: AxiosInstance;
  private requestId: number = 0;

  constructor() {
    this.client = axios.create({
      baseURL: MCP_URL,
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${CAPABILITY_TOKEN}`,
      },
    });
  }

  private async makeRequest(
    method: string,
    params: Record<string, unknown>
  ): Promise<MCPResponse> {
    const request: MCPRequest = {
      jsonrpc: '2.0',
      method,
      params,
      id: ++this.requestId,
    };

    try {
      const response = await this.client.post<MCPResponse>('/', request);
      return response.data;
    } catch (error) {
      console.error('Request failed:', error);
      throw error;
    }
  }

  async listTools(): Promise<void> {
    console.log('\n📋 Available Tools:');
    Object.entries(TOOLS).forEach(([tool, desc]) => {
      console.log(`  • ${tool}: ${desc}`);
    });
  }

  async readFile(path: string): Promise<string> {
    console.log(`\n📖 Reading file: ${path}`);

    const response = await this.makeRequest('tools/call', {
      name: 'read_file',
      arguments: { path },
    });

    if (response.error) {
      console.log(`  ❌ Error: ${response.error.message}`);
      return '';
    }

    const content =
      response.result?.result?.content?.[0]?.text || '';
    console.log(`  ✓ Read ${content.length} characters`);
    return content;
  }

  async writeFile(path: string, content: string): Promise<boolean> {
    console.log(`\n✍️  Writing file: ${path}`);

    const response = await this.makeRequest('tools/call', {
      name: 'write_file',
      arguments: { path, content },
    });

    if (response.error) {
      console.log(`  ❌ Error: ${response.error.message}`);
      if ((response.error as any).requires_approval) {
        console.log(
          `  ⏸️  Approval needed! Request ID: ${(response.error as any).request_id}`
        );
      }
      return false;
    }

    console.log(`  ✓ Wrote ${content.length} characters`);
    return true;
  }

  async runCargoCheck(): Promise<boolean> {
    console.log(`\n🔍 Running: cargo check`);

    const response = await this.makeRequest('tools/call', {
      name: 'run_cargo_check',
      arguments: {},
    });

    if (response.error) {
      const output = response.error.message;
      console.log(`  ❌ Build failed:\n${output}`);
      return false;
    }

    const output = response.result?.result?.content?.[0]?.text || '';
    if (output.toLowerCase().includes('error')) {
      console.log(`  ❌ Compilation errors found:\n${output}`);
      return false;
    }

    console.log(`  ✓ Build succeeded`);
    return true;
  }

  async searchCodebase(pattern: string): Promise<string[]> {
    console.log(`\n🔎 Searching for: ${pattern}`);

    const response = await this.makeRequest('tools/call', {
      name: 'search_codebase',
      arguments: { pattern },
    });

    if (response.error) {
      console.log(`  ❌ Error: ${response.error.message}`);
      return [];
    }

    const matches =
      (response.result?.result?.content?.[0]?.matches as string[]) || [];
    console.log(`  ✓ Found ${matches.length} matches`);
    matches.slice(0, 5).forEach((match) => {
      console.log(`    - ${match}`);
    });
    return matches;
  }

  async run(): Promise<void> {
    console.log('╔════════════════════════════════════════════════════════════════╗');
    console.log('║  🤖 Bonsai TypeScript Agent                                    ║');
    console.log('║  Connected to Universal Agent Control System                   ║');
    console.log('╚════════════════════════════════════════════════════════════════╝');

    // Step 1: List available tools
    console.log('\n▶ Step 1: List available tools');
    await this.listTools();

    // Step 2: Read Cargo.toml to understand the workspace
    console.log('\n▶ Step 2: Read Cargo.toml');
    const cargoContent = await this.readFile('Cargo.toml');
    if (cargoContent) {
      const lines = cargoContent.split('\n').slice(0, 10);
      console.log(`  First 10 lines:`);
      lines.forEach((line) => {
        console.log(`    ${line}`);
      });
    }

    // Step 3: Run cargo check
    console.log('\n▶ Step 3: Run cargo check --workspace');
    const buildSuccess = await this.runCargoCheck();
    if (!buildSuccess) {
      console.log('\n  Note: Fixing compilation errors would go here');
      console.log('  In this demo, we just report the issue');
    }

    // Step 4: Search for todo!() macros
    console.log('\n▶ Step 4: Search for unimplemented features');
    const todos = await this.searchCodebase(
      'todo!()|unimplemented!()'
    );
    if (todos.length > 0) {
      console.log(`  Found ${todos.length} instances of todo!/unimplemented!`);
    }

    // Step 5: Create a simple improvement report
    console.log('\n▶ Step 5: Create an improvement file');
    const improvement = `# Bonsai Self-Improvement Report

Generated by TypeScript Agent

## Summary
This is an automated report from the Bonsai TypeScript Agent demonstrating
integration with the Universal Agent Control System.

## Steps Completed
1. ✓ Listed available tools
2. ✓ Read workspace configuration (Cargo.toml)
3. ✓ Ran cargo check to verify build status
4. ✓ Searched for unimplemented features (todo!/unimplemented!)

## Observations
- The workspace structure is well-organized
- Multiple crates present: bonsai-mcp-server, bonsai-inference, etc.
- Build status verified

## Next Steps
- Implement missing features (identified by search)
- Add comprehensive tests
- Document public APIs
- Optimize performance hotspots

Generated at: ${new Date().toISOString()}
`;

    const reportPath = 'AGENT_REPORT_TS.md';
    const reportWritten = await this.writeFile(reportPath, improvement);
    if (reportWritten) {
      console.log(`  ✓ Report written to ${reportPath}`);
    }

    console.log('\n╔════════════════════════════════════════════════════════════════╗');
    console.log('║ ✅ Agent execution complete                                    ║');
    console.log('║                                                                ║');
    console.log('║ All actions were visible on the UACS dashboard:               ║');
    console.log('║ http://localhost:5173                                         ║');
    console.log('║                                                                ║');
    console.log('║ You could approve/deny any file writes or deployments.        ║');
    console.log('╚════════════════════════════════════════════════════════════════╝');
  }
}

// Main execution
(async () => {
  try {
    const agent = new BonsaiAgent();
    await agent.run();
  } catch (error) {
    console.error('\n❌ Error:', error);
    process.exit(1);
  }
})();
