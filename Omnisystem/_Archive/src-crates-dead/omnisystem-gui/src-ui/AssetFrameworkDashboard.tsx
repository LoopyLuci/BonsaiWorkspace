import React, { useState, useEffect } from 'react';
import './AssetFrameworkDashboard.css';

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

interface Asset {
  asset_id: string;
  name: string;
  description: string;
  asset_type: 'WebComponent' | 'GameAsset' | 'VisualAsset' | 'AudioAsset';
  category: string;
  rating: number;
  download_count: number;
  usage_count: number;
  tags: string[];
  file_path: string;
  file_size_bytes: number;
  created_timestamp: number;
}

interface GenerationRequest {
  prompt: string;
  asset_type: string;
  generation_method: 'Procedural' | 'Deterministic' | 'Hybrid' | 'AIAssisted';
  complexity: 'Simple' | 'Moderate' | 'Complex';
  parameters?: Record<string, any>;
}

interface SystemStats {
  total_assets: number;
  total_libraries: number;
  total_downloads: number;
  average_rating: number;
  total_usage: number;
  total_storage_bytes: number;
}

// ============================================================================
// ASSET FRAMEWORK DASHBOARD
// ============================================================================

export default function AssetFrameworkDashboard() {
  const [activeTab, setActiveTab] = useState<'library' | 'generate' | 'stats' | 'manage'>('library');
  const [assets, setAssets] = useState<Asset[]>([]);
  const [filteredAssets, setFilteredAssets] = useState<Asset[]>([]);
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedAsset, setSelectedAsset] = useState<Asset | null>(null);
  const [assetTypeFilter, setAssetTypeFilter] = useState<string>('all');
  const [sortBy, setSortBy] = useState<'name' | 'rating' | 'downloads' | 'date'>('rating');
  const [generationForm, setGenerationForm] = useState<GenerationRequest>({
    prompt: '',
    asset_type: 'WebComponent',
    generation_method: 'Procedural',
    complexity: 'Moderate',
  });
  const [generationStatus, setGenerationStatus] = useState<'idle' | 'generating' | 'success' | 'error'>('idle');
  const [generationProgress, setGenerationProgress] = useState(0);

  // Load initial data
  useEffect(() => {
    loadAssets();
    loadStats();
  }, []);

  // Filter and sort assets
  useEffect(() => {
    let filtered = assets;

    // Filter by search term
    if (searchTerm) {
      filtered = filtered.filter(a =>
        a.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.description.toLowerCase().includes(searchTerm.toLowerCase()) ||
        a.tags.some(t => t.toLowerCase().includes(searchTerm.toLowerCase()))
      );
    }

    // Filter by asset type
    if (assetTypeFilter !== 'all') {
      filtered = filtered.filter(a => a.asset_type === assetTypeFilter);
    }

    // Sort
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'name':
          return a.name.localeCompare(b.name);
        case 'rating':
          return b.rating - a.rating;
        case 'downloads':
          return b.download_count - a.download_count;
        case 'date':
          return b.created_timestamp - a.created_timestamp;
        default:
          return 0;
      }
    });

    setFilteredAssets(filtered);
  }, [assets, searchTerm, assetTypeFilter, sortBy]);

  // Load assets from backend
  const loadAssets = async () => {
    try {
      const response = await fetch('/api/v1/assets');
      const data = await response.json();
      setAssets(data.assets || []);
    } catch (error) {
      console.error('Error loading assets:', error);
    }
  };

  // Load statistics
  const loadStats = async () => {
    try {
      const response = await fetch('/api/v1/assets/stats');
      const data = await response.json();
      setStats(data);
    } catch (error) {
      console.error('Error loading stats:', error);
    }
  };

  // Handle asset generation
  const handleGenerateAsset = async () => {
    if (!generationForm.prompt) {
      alert('Please enter a prompt');
      return;
    }

    setGenerationStatus('generating');
    setGenerationProgress(0);

    try {
      const response = await fetch('/api/v1/assets/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(generationForm),
      });

      if (response.ok) {
        setGenerationProgress(100);
        setGenerationStatus('success');
        setGenerationForm({ prompt: '', asset_type: 'WebComponent', generation_method: 'Procedural', complexity: 'Moderate' });
        setTimeout(() => {
          setGenerationStatus('idle');
          loadAssets();
        }, 2000);
      }
    } catch (error) {
      setGenerationStatus('error');
      console.error('Generation error:', error);
    }
  };

  // Render Library Tab
  const renderLibraryTab = () => (
    <div className="afd-library">
      <div className="afd-header">
        <h2>Asset Library</h2>
        <p>Browse and manage {filteredAssets.length} assets</p>
      </div>

      <div className="afd-controls">
        <div className="search-bar">
          <input
            type="text"
            placeholder="Search assets..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="search-input"
          />
        </div>

        <div className="filters">
          <select
            value={assetTypeFilter}
            onChange={(e) => setAssetTypeFilter(e.target.value)}
            className="filter-select"
          >
            <option value="all">All Types</option>
            <option value="WebComponent">Web Components</option>
            <option value="GameAsset">Game Assets</option>
            <option value="VisualAsset">Visual Assets</option>
            <option value="AudioAsset">Audio Assets</option>
          </select>

          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
            className="filter-select"
          >
            <option value="rating">Sort by Rating</option>
            <option value="downloads">Sort by Downloads</option>
            <option value="date">Sort by Date</option>
            <option value="name">Sort by Name</option>
          </select>
        </div>
      </div>

      <div className="assets-container">
        <div className="assets-grid">
          {filteredAssets.map((asset) => (
            <div
              key={asset.asset_id}
              className={`asset-card ${selectedAsset?.asset_id === asset.asset_id ? 'selected' : ''}`}
              onClick={() => setSelectedAsset(asset)}
            >
              <div className="asset-type-badge">{asset.asset_type}</div>
              <h4>{asset.name}</h4>
              <p className="asset-desc">{asset.description}</p>
              <div className="asset-stats-row">
                <span className="stat">⭐ {asset.rating.toFixed(1)}</span>
                <span className="stat">📥 {asset.download_count}</span>
                <span className="stat">▶ {asset.usage_count}</span>
              </div>
              <div className="asset-tags">
                {asset.tags.slice(0, 3).map((tag) => (
                  <span key={tag} className="tag">{tag}</span>
                ))}
              </div>
            </div>
          ))}
        </div>

        {selectedAsset && (
          <div className="asset-detail-panel">
            <div className="detail-header">
              <h3>{selectedAsset.name}</h3>
              <button className="close-btn" onClick={() => setSelectedAsset(null)}>✕</button>
            </div>
            <div className="detail-content">
              <div className="detail-row">
                <span className="label">Type:</span>
                <span className="value">{selectedAsset.asset_type}</span>
              </div>
              <div className="detail-row">
                <span className="label">Category:</span>
                <span className="value">{selectedAsset.category}</span>
              </div>
              <div className="detail-row">
                <span className="label">Rating:</span>
                <span className="value">⭐ {selectedAsset.rating.toFixed(1)}/5.0</span>
              </div>
              <div className="detail-row">
                <span className="label">Downloads:</span>
                <span className="value">{selectedAsset.download_count}</span>
              </div>
              <div className="detail-row">
                <span className="label">Size:</span>
                <span className="value">{(selectedAsset.file_size_bytes / 1024).toFixed(1)} KB</span>
              </div>
              <div className="detail-full">
                <span className="label">Description:</span>
                <p>{selectedAsset.description}</p>
              </div>
              <div className="detail-full">
                <span className="label">Tags:</span>
                <div className="tags-list">
                  {selectedAsset.tags.map((tag) => (
                    <span key={tag} className="tag-item">{tag}</span>
                  ))}
                </div>
              </div>
              <div className="detail-actions">
                <button className="action-btn">Edit</button>
                <button className="action-btn">Download</button>
                <button className="action-btn primary">Use Asset</button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );

  // Render Generate Tab
  const renderGenerateTab = () => (
    <div className="afd-generate">
      <div className="afd-header">
        <h2>Asset Generator</h2>
        <p>Create new assets using AI, procedural, or hybrid methods</p>
      </div>

      <div className="generation-form">
        <div className="form-group">
          <label>Asset Type</label>
          <select
            value={generationForm.asset_type}
            onChange={(e) => setGenerationForm({ ...generationForm, asset_type: e.target.value })}
          >
            <option value="WebComponent">Web Component</option>
            <option value="GameAsset">Game Asset</option>
            <option value="VisualAsset">Visual Asset</option>
            <option value="AudioAsset">Audio Asset</option>
          </select>
        </div>

        <div className="form-group">
          <label>Generation Method</label>
          <select
            value={generationForm.generation_method}
            onChange={(e) => setGenerationForm({ ...generationForm, generation_method: e.target.value as any })}
          >
            <option value="Procedural">Procedural</option>
            <option value="Deterministic">Deterministic (Reproducible)</option>
            <option value="Hybrid">Hybrid (Procedural + AI)</option>
            <option value="AIAssisted">AI-Assisted</option>
          </select>
        </div>

        <div className="form-group">
          <label>Complexity</label>
          <select
            value={generationForm.complexity}
            onChange={(e) => setGenerationForm({ ...generationForm, complexity: e.target.value as any })}
          >
            <option value="Simple">Simple</option>
            <option value="Moderate">Moderate</option>
            <option value="Complex">Complex</option>
          </select>
        </div>

        <div className="form-group full-width">
          <label>Prompt / Description</label>
          <textarea
            value={generationForm.prompt}
            onChange={(e) => setGenerationForm({ ...generationForm, prompt: e.target.value })}
            placeholder="Describe the asset you want to generate..."
            className="prompt-textarea"
            rows={5}
          />
        </div>

        {generationStatus === 'generating' && (
          <div className="progress-container">
            <div className="progress-bar">
              <div className="progress-fill" style={{ width: `${generationProgress}%` }}></div>
            </div>
            <p>Generating asset... {generationProgress}%</p>
          </div>
        )}

        {generationStatus === 'success' && (
          <div className="success-message">✅ Asset generated successfully!</div>
        )}

        {generationStatus === 'error' && (
          <div className="error-message">❌ Generation failed. Please try again.</div>
        )}

        <button
          className="generate-btn"
          onClick={handleGenerateAsset}
          disabled={generationStatus === 'generating'}
        >
          {generationStatus === 'generating' ? 'Generating...' : 'Generate Asset'}
        </button>
      </div>

      <div className="generation-info">
        <h3>About Generation Methods</h3>
        <div className="info-grid">
          <div className="info-card">
            <h4>Procedural</h4>
            <p>Rule-based generation for consistent, structured assets</p>
          </div>
          <div className="info-card">
            <h4>Deterministic</h4>
            <p>Seed-based generation for reproducible results</p>
          </div>
          <div className="info-card">
            <h4>Hybrid</h4>
            <p>Combines procedural rules with AI enhancement</p>
          </div>
          <div className="info-card">
            <h4>AI-Assisted</h4>
            <p>Natural language to asset using advanced AI models</p>
          </div>
        </div>
      </div>
    </div>
  );

  // Render Stats Tab
  const renderStatsTab = () => (
    <div className="afd-stats">
      <div className="afd-header">
        <h2>System Statistics</h2>
        <p>Overview of asset framework metrics</p>
      </div>

      {stats && (
        <div className="stats-grid">
          <div className="stat-card">
            <div className="stat-number">{stats.total_assets}</div>
            <div className="stat-label">Total Assets</div>
          </div>
          <div className="stat-card">
            <div className="stat-number">{stats.total_libraries}</div>
            <div className="stat-label">Libraries</div>
          </div>
          <div className="stat-card">
            <div className="stat-number">{stats.total_downloads}</div>
            <div className="stat-label">Total Downloads</div>
          </div>
          <div className="stat-card">
            <div className="stat-number">{stats.average_rating.toFixed(1)}</div>
            <div className="stat-label">Avg Rating</div>
          </div>
          <div className="stat-card">
            <div className="stat-number">{stats.total_usage}</div>
            <div className="stat-label">Total Usage</div>
          </div>
          <div className="stat-card">
            <div className="stat-number">{(stats.total_storage_bytes / 1000000000).toFixed(1)} GB</div>
            <div className="stat-label">Storage Used</div>
          </div>
        </div>
      )}

      <div className="stats-charts">
        <div className="chart-placeholder">
          <h3>Assets by Type</h3>
          <div style={{ height: '200px', background: 'rgba(0, 212, 255, 0.05)', borderRadius: '8px' }}>
            Chart placeholder
          </div>
        </div>
        <div className="chart-placeholder">
          <h3>Download Trends</h3>
          <div style={{ height: '200px', background: 'rgba(0, 212, 255, 0.05)', borderRadius: '8px' }}>
            Chart placeholder
          </div>
        </div>
      </div>
    </div>
  );

  // Render Manage Tab
  const renderManageTab = () => (
    <div className="afd-manage">
      <div className="afd-header">
        <h2>Asset Management</h2>
        <p>Manage libraries, versions, and metadata</p>
      </div>

      <div className="manage-section">
        <h3>Libraries</h3>
        <div className="libraries-list">
          {['Web Assets', 'Game Assets', 'Visual Assets', 'Audio Assets'].map((lib) => (
            <div key={lib} className="library-item">
              <div className="lib-icon">📚</div>
              <div className="lib-info">
                <h4>{lib}</h4>
                <p>Manage and organize assets</p>
              </div>
              <button className="manage-btn">Manage</button>
            </div>
          ))}
        </div>
      </div>

      <div className="manage-section">
        <h3>Quick Actions</h3>
        <div className="actions-grid">
          <button className="action-card">
            <div className="action-icon">📦</div>
            <div>Batch Import</div>
          </button>
          <button className="action-card">
            <div className="action-icon">💾</div>
            <div>Backup Assets</div>
          </button>
          <button className="action-card">
            <div className="action-icon">🔄</div>
            <div>Sync Cache</div>
          </button>
          <button className="action-card">
            <div className="action-icon">⚙️</div>
            <div>Settings</div>
          </button>
        </div>
      </div>
    </div>
  );

  return (
    <div className="asset-framework-dashboard">
      <div className="afd-navigation">
        <button
          className={`nav-tab ${activeTab === 'library' ? 'active' : ''}`}
          onClick={() => setActiveTab('library')}
        >
          📚 Library
        </button>
        <button
          className={`nav-tab ${activeTab === 'generate' ? 'active' : ''}`}
          onClick={() => setActiveTab('generate')}
        >
          ✨ Generate
        </button>
        <button
          className={`nav-tab ${activeTab === 'stats' ? 'active' : ''}`}
          onClick={() => setActiveTab('stats')}
        >
          📊 Statistics
        </button>
        <button
          className={`nav-tab ${activeTab === 'manage' ? 'active' : ''}`}
          onClick={() => setActiveTab('manage')}
        >
          ⚙️ Manage
        </button>
      </div>

      <div className="afd-content">
        {activeTab === 'library' && renderLibraryTab()}
        {activeTab === 'generate' && renderGenerateTab()}
        {activeTab === 'stats' && renderStatsTab()}
        {activeTab === 'manage' && renderManageTab()}
      </div>
    </div>
  );
}
