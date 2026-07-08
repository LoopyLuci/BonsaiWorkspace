<script lang="ts">
	import { onMount } from 'svelte';
	import type { Diagnostic, Severity } from '$lib/types';

	interface LintDiagnostic {
		rule_id: string;
		message: string;
		severity: 'note' | 'hint' | 'warning' | 'error' | 'fatal';
		file: string;
		line: number;
		column: number;
		fix?: string;
	}

	interface LintSummary {
		total_diagnostics: number;
		by_severity: Record<string, number>;
		files_scanned: number;
		duration_ms: number;
	}

	let diagnostics: LintDiagnostic[] = [];
	let summary: LintSummary | null = null;
	let selectedDiagnostic: LintDiagnostic | null = null;
	let isLinting = false;
	let filterBySeverity: string | null = null;
	let sortBy: 'severity' | 'file' | 'rule' = 'severity';
	let showDetails = false;

	onMount(async () => {
		setupWebSocketListener();
	});

	function setupWebSocketListener() {
		// Subscribe to linting events from MCP server
		const ws = new WebSocket('ws://localhost:8080/lint-events');

		ws.onmessage = (event) => {
			const data = JSON.parse(event.data);
			if (data.type === 'lint-completed') {
				diagnostics = data.diagnostics || [];
				summary = data.summary || null;
				isLinting = false;
			} else if (data.type === 'lint-started') {
				isLinting = true;
			}
		};

		ws.onerror = (error) => {
			console.error('WebSocket error:', error);
		};
	}

	async function runLint() {
		isLinting = true;
		try {
			const response = await fetch('/api/tools/workspace_lint_repo', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					exclude_patterns: ['target/**', 'node_modules/**'],
					confidence_threshold: 0.7,
					ai_filtering: true,
					spell_check: true,
				}),
			});

			const result = await response.json();
			diagnostics = result.diagnostics || [];
			summary = result.summary || null;
		} catch (error) {
			console.error('Linting error:', error);
		} finally {
			isLinting = false;
		}
	}

	async function runLintOnFile(filePath: string) {
		try {
			const response = await fetch('/api/tools/workspace_lint_file', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					path: filePath,
					confidence_threshold: 0.7,
				}),
			});

			const result = await response.json();
			diagnostics = result.diagnostics || [];
			summary = result.summary || null;
		} catch (error) {
			console.error('Single file linting error:', error);
		}
	}

	async function explainDiagnostic(diag: LintDiagnostic) {
		try {
			const response = await fetch('/api/tools/workspace_explain_diagnostic', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					rule_id: diag.rule_id,
					code_snippet: diag.message,
					language: 'rust', // TODO: detect from file extension
					message: diag.message,
				}),
			});

			const explanation = await response.json();
			selectedDiagnostic = { ...diag, fix: explanation.explanation };
			showDetails = true;
		} catch (error) {
			console.error('Explanation error:', error);
		}
	}

	async function reportFalsePositive(diag: LintDiagnostic, explanation: string) {
		try {
			await fetch('/api/tools/workspace_report_false_positive', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					rule_id: diag.rule_id,
					file: diag.file,
					line: diag.line,
					explanation: explanation,
				}),
			});
			console.log('False positive reported:', diag.rule_id);
		} catch (error) {
			console.error('Error reporting false positive:', error);
		}
	}

	async function dismissDiagnostic(diag: LintDiagnostic) {
		try {
			await fetch('/api/tools/workspace_dismiss_diagnostic', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					rule_id: diag.rule_id,
					file: diag.file,
					line: diag.line,
				}),
			});
			console.log('Diagnostic dismissed:', diag.rule_id);
		} catch (error) {
			console.error('Error dismissing diagnostic:', error);
		}
	}

	async function acceptAndApplyFix(diag: LintDiagnostic) {
		try {
			await fetch('/api/tools/workspace_apply_fix', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					rule_id: diag.rule_id,
					file: diag.file,
					line: diag.line,
					fix: diag.fix,
				}),
			});
			console.log('Fix applied:', diag.rule_id);
		} catch (error) {
			console.error('Error applying fix:', error);
		}
	}

	function getSeverityColor(severity: string): string {
		switch (severity) {
			case 'error':
			case 'fatal':
				return '#dc2626';
			case 'warning':
				return '#ea580c';
			case 'hint':
				return '#3b82f6';
			case 'note':
			default:
				return '#6366f1';
		}
	}

	function getSeverityIcon(severity: string): string {
		switch (severity) {
			case 'error':
				return '⛔';
			case 'fatal':
				return '💥';
			case 'warning':
				return '⚠️';
			case 'hint':
				return '💡';
			case 'note':
			default:
				return 'ℹ️';
		}
	}

	function applyFilter() {
		if (!filterBySeverity) return diagnostics;
		return diagnostics.filter((d) => d.severity === filterBySeverity);
	}

	function applySorting(items: LintDiagnostic[]) {
		const sorted = [...items];
		switch (sortBy) {
			case 'severity':
				return sorted.sort((a, b) => {
					const severityOrder = { fatal: 5, error: 4, warning: 3, hint: 2, note: 1 };
					return (severityOrder[b.severity] || 0) - (severityOrder[a.severity] || 0);
				});
			case 'file':
				return sorted.sort((a, b) => a.file.localeCompare(b.file));
			case 'rule':
				return sorted.sort((a, b) => a.rule_id.localeCompare(b.rule_id));
			default:
				return sorted;
		}
	}

	$: filteredDiagnostics = applySorting(applyFilter());
</script>

<div class="lint-panel">
	<div class="header">
		<h2>Lint Results</h2>
		<div class="controls">
			<button on:click={runLint} disabled={isLinting} class="btn-primary">
				{isLinting ? '🔄 Linting...' : '▶ Lint Repository'}
			</button>

			<select bind:value={filterBySeverity} class="filter-select">
				<option value={null}>All Severities</option>
				<option value="error">Errors</option>
				<option value="warning">Warnings</option>
				<option value="hint">Hints</option>
				<option value="note">Notes</option>
			</select>

			<select bind:value={sortBy} class="sort-select">
				<option value="severity">Sort by Severity</option>
				<option value="file">Sort by File</option>
				<option value="rule">Sort by Rule</option>
			</select>
		</div>
	</div>

	{#if summary}
		<div class="summary">
			<div class="stat">
				<span class="label">Total Diagnostics:</span>
				<span class="value">{summary.total_diagnostics}</span>
			</div>
			<div class="stat">
				<span class="label">Files Scanned:</span>
				<span class="value">{summary.files_scanned}</span>
			</div>
			<div class="stat">
				<span class="label">Duration:</span>
				<span class="value">{summary.duration_ms}ms</span>
			</div>
			{#each Object.entries(summary.by_severity) as [severity, count]}
				<div class="stat severity-stat">
					<span class="label">{severity}:</span>
					<span class="value" style="color: {getSeverityColor(severity)}">
						{getSeverityIcon(severity)} {count}
					</span>
				</div>
			{/each}
		</div>
	{/if}

	<div class="diagnostics-list">
		{#if filteredDiagnostics.length === 0}
			<p class="no-diagnostics">
				{diagnostics.length === 0 ? '✅ No issues found!' : 'No diagnostics match the current filter.'}
			</p>
		{:else}
			{#each filteredDiagnostics as diag (diag.rule_id + diag.file + diag.line)}
				<div
					class="diagnostic-item"
					class:selected={selectedDiagnostic === diag}
					on:click={() => {
						selectedDiagnostic = diag;
						showDetails = true;
					}}
				>
					<div class="diagnostic-header">
						<span class="severity-badge" style="background-color: {getSeverityColor(diag.severity)}">
							{getSeverityIcon(diag.severity)} {diag.severity}
						</span>
						<span class="rule-id">{diag.rule_id}</span>
						<span class="location">
							{diag.file}:{diag.line}:{diag.column}
						</span>
					</div>
					<div class="message">{diag.message}</div>
					<div class="fix-suggestion">
						<button on:click={() => explainDiagnostic(diag)} class="btn-small">
							💡 Explain
						</button>
						{#if diag.fix}
							<button on:click={() => acceptAndApplyFix(diag)} class="btn-small btn-success">
								✓ Apply Fix
							</button>
						{/if}
						<button on:click={() => reportFalsePositive(diag, 'Not applicable')} class="btn-small btn-warning">
							✕ False Positive
						</button>
						<button on:click={() => dismissDiagnostic(diag)} class="btn-small btn-dismiss">
							→ Dismiss
						</button>
					</div>
				</div>
			{/each}
		{/if}
	</div>

	{#if showDetails && selectedDiagnostic}
		<div class="details-panel">
			<div class="details-header">
				<h3>Diagnostic Details</h3>
				<button on:click={() => (showDetails = false)} class="btn-close">✕</button>
			</div>
			<div class="details-content">
				<div class="detail-item">
					<span class="detail-label">Rule ID:</span>
					<span class="detail-value">{selectedDiagnostic.rule_id}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">File:</span>
					<span class="detail-value">{selectedDiagnostic.file}:{selectedDiagnostic.line}:{selectedDiagnostic.column}</span>
				</div>
				<div class="detail-item">
					<span class="detail-label">Message:</span>
					<span class="detail-value">{selectedDiagnostic.message}</span>
				</div>
				{#if selectedDiagnostic.fix}
					<div class="detail-item">
						<span class="detail-label">Fix:</span>
						<span class="detail-value">{selectedDiagnostic.fix}</span>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.lint-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		background-color: #0f1419;
		color: #e0e0e0;
		font-family: 'Monaco', 'Courier New', monospace;
	}

	.header {
		padding: 12px;
		border-bottom: 1px solid #333;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header h2 {
		margin: 0;
		font-size: 16px;
		color: #7aa2f7;
	}

	.controls {
		display: flex;
		gap: 8px;
		align-items: center;
	}

	.btn-primary,
	.filter-select,
	.sort-select {
		padding: 6px 12px;
		background-color: #1a1b26;
		color: #e0e0e0;
		border: 1px solid #333;
		border-radius: 4px;
		cursor: pointer;
		font-size: 12px;
	}

	.btn-primary:hover {
		background-color: #2d2e3e;
	}

	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.summary {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 8px;
		padding: 12px;
		background-color: #1a1b26;
		border-bottom: 1px solid #333;
	}

	.stat {
		display: flex;
		justify-content: space-between;
		font-size: 12px;
	}

	.stat .label {
		color: #888;
	}

	.stat .value {
		font-weight: bold;
		color: #9ece6a;
	}

	.severity-stat .value {
		font-weight: bold;
	}

	.diagnostics-list {
		flex: 1;
		overflow-y: auto;
		padding: 8px;
	}

	.no-diagnostics {
		padding: 20px;
		text-align: center;
		color: #888;
	}

	.diagnostic-item {
		padding: 8px;
		margin-bottom: 4px;
		background-color: #1a1b26;
		border-left: 3px solid #ff9500;
		border-radius: 4px;
		cursor: pointer;
		transition: background-color 0.15s;
	}

	.diagnostic-item:hover {
		background-color: #252637;
	}

	.diagnostic-item.selected {
		background-color: #2d2e3e;
		border-left-color: #7aa2f7;
	}

	.diagnostic-header {
		display: flex;
		gap: 8px;
		align-items: center;
		margin-bottom: 4px;
	}

	.severity-badge {
		padding: 2px 6px;
		border-radius: 3px;
		font-size: 11px;
		color: white;
		font-weight: bold;
	}

	.rule-id {
		color: #7aa2f7;
		font-size: 12px;
		font-weight: bold;
	}

	.location {
		color: #888;
		font-size: 11px;
		margin-left: auto;
	}

	.message {
		color: #dcdcdc;
		font-size: 12px;
		margin-bottom: 4px;
	}

	.fix-suggestion {
		display: flex;
		gap: 4px;
		margin-top: 4px;
	}

	.btn-small {
		padding: 4px 8px;
		font-size: 11px;
		background-color: #2d2e3e;
		color: #e0e0e0;
		border: 1px solid #444;
		border-radius: 3px;
		cursor: pointer;
	}

	.btn-small:hover {
		background-color: #3d3e4e;
	}

	.btn-success {
		background-color: #2d5a2d;
		border-color: #4a7c4a;
	}

	.btn-success:hover {
		background-color: #3d7a3d;
	}

	.btn-warning {
		background-color: #5a4a2d;
		border-color: #7c6a4a;
	}

	.btn-warning:hover {
		background-color: #7a6a3d;
	}

	.btn-dismiss {
		background-color: #4a4a5a;
		border-color: #6a6a7c;
	}

	.btn-dismiss:hover {
		background-color: #6a6a7a;
	}

	.details-panel {
		border-top: 1px solid #333;
		background-color: #1a1b26;
		padding: 12px;
		max-height: 200px;
		overflow-y: auto;
	}

	.details-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.details-header h3 {
		margin: 0;
		font-size: 14px;
		color: #7aa2f7;
	}

	.btn-close {
		background: none;
		border: none;
		color: #888;
		cursor: pointer;
		font-size: 16px;
	}

	.btn-close:hover {
		color: #e0e0e0;
	}

	.details-content {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.detail-item {
		display: flex;
		gap: 8px;
		font-size: 12px;
	}

	.detail-label {
		color: #888;
		min-width: 100px;
	}

	.detail-value {
		color: #dcdcdc;
		flex: 1;
		word-break: break-word;
	}
</style>
