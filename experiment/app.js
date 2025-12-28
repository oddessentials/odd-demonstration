/**
 * Experiment Viewer - AI Model Assessment Comparison Tool
 * A GitHub Pages-compatible viewer for markdown and PDF assessments
 */

const ExperimentViewer = {
    // Corrected file tree matching actual directory structure
    fileTree: [
        {
            name: 'control-groups',
            type: 'dir',
            path: 'control-groups',
            children: [
                {
                    name: 'dapr',
                    type: 'dir',
                    path: 'control-groups/dapr',
                    children: [
                        { name: 'dapr-claude-opus-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-claude-opus-assessment-2025-12-27.md' },
                        { name: 'dapr-claude-sonnet-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-claude-sonnet-assessment-2025-12-27.md' },
                        { name: 'dapr-gemini-flash-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-gemini-flash-assessment-2025-12-27.md' },
                        { name: 'dapr-gemini-high-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-gemini-high-assessment-2025-12-27.md' },
                        { name: 'dapr-gpt-oss-120b-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-gpt-oss-120b-assessment-2025-12-27.md' },
                        { name: 'dapr-gpt5.2-browser-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-gpt5.2-browser-assessment-2025-12-27.md' },
                        { name: 'dapr-gpt5.2-browser-assessment-2025-12-27.pdf', type: 'file', path: 'control-groups/dapr/dapr-gpt5.2-browser-assessment-2025-12-27.pdf' },
                        { name: 'dapr-supergrok-browser-assessment-2025-12-27.md', type: 'file', path: 'control-groups/dapr/dapr-supergrok-browser-assessment-2025-12-27.md' },
                    ]
                },
                {
                    name: 'google-microservices-demo',
                    type: 'dir',
                    path: 'control-groups/google-microservices-demo',
                    children: [
                        { name: 'gm-claude-opus-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-claude-opus-assessment-2025-12-27.md' },
                        { name: 'gm-claude-sonnet-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-claude-sonnet-assessment-2025-12-27.md' },
                        { name: 'gm-gemini-flash-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-gemini-flash-assessment-2025-12-27.md' },
                        { name: 'gm-gemini-high-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-gemini-high-assessment-2025-12-27.md' },
                        { name: 'gm-gpt-oss-120b-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-gpt-oss-120b-assessment-2025-12-27.md' },
                        { name: 'gm-gpt5.2-browser-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-gpt5.2-browser-assessment-2025-12-27.md' },
                        { name: 'gm-gpt5.2-browser-assessment-2025-12-27.pdf', type: 'file', path: 'control-groups/google-microservices-demo/gm-gpt5.2-browser-assessment-2025-12-27.pdf' },
                        { name: 'gm-supergrok-browser-assessment-2025-12-27.md', type: 'file', path: 'control-groups/google-microservices-demo/gm-supergrok-browser-assessment-2025-12-27.md' },
                    ]
                }
            ]
        },
        {
            name: 'experiment-group',
            type: 'dir',
            path: 'experiment-group',
            children: [
                { name: 'oed-claude-opus-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-claude-opus-assessment-2025-12-27.md' },
                { name: 'oed-claude-sonnet-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-claude-sonnet-assessment-2025-12-27.md' },
                { name: 'oed-gemini-flash-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-gemini-flash-assessment-2025-12-27.md' },
                { name: 'oed-gemini-high-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-gemini-high-assessment-2025-12-27.md' },
                { name: 'oed-gpt-codex-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-gpt-codex-assessment-2025-12-27.md' },
                { name: 'oed-gpt-oss-120b-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-gpt-oss-120b-assessment-2025-12-27.md' },
                { name: 'oed-gpt5.2-browser-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-gpt5.2-browser-assessment-2025-12-27.md' },
                { name: 'oed-gpt5.2-browser-assessment-2025-12-27.pdf', type: 'file', path: 'experiment-group/oed-gpt5.2-browser-assessment-2025-12-27.pdf' },
                { name: 'oed-supergrok-browser-assessment-2025-12-27.md', type: 'file', path: 'experiment-group/oed-supergrok-browser-assessment-2025-12-27.md' },
            ]
        },
        { name: 'experiment.md', type: 'file', path: 'experiment.md' },
        { name: 'experiment.pdf', type: 'file', path: 'experiment.pdf' },
    ],

    // Application state
    state: {
        currentFile: null,
        compareFile: null,
        compareMode: false,
    },

    // Icons
    icons: {
        folder: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>`,
        folderOpen: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2v1"></path><path d="M4 9h18l-2 10H6L4 9z"></path></svg>`,
        markdown: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>`,
        pdf: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><path d="M9 15v-2h2a1 1 0 1 1 0 2H9z"></path></svg>`,
        compare: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="18" rx="1"></rect><rect x="14" y="3" width="7" height="18" rx="1"></rect></svg>`,
        close: `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>`,
    },

    // Build file tree recursively
    buildTree(items, parentUl, paneId = 'primary') {
        items.sort((a, b) => {
            if (a.type === b.type) return a.name.localeCompare(b.name);
            return a.type === 'dir' ? -1 : 1;
        });

        for (const item of items) {
            const li = document.createElement('li');

            if (item.type === 'dir') {
                const span = document.createElement('span');
                span.className = 'folder';
                span.innerHTML = `${this.icons.folder}<span class="name">${item.name}</span>`;
                li.appendChild(span);

                const ul = document.createElement('ul');
                ul.className = 'collapsed';
                li.appendChild(ul);

                if (item.children) {
                    this.buildTree(item.children, ul, paneId);
                }

                span.onclick = () => {
                    const isExpanded = !ul.classList.contains('collapsed');
                    ul.classList.toggle('collapsed');
                    span.innerHTML = `${isExpanded ? this.icons.folder : this.icons.folderOpen}<span class="name">${item.name}</span>`;
                };
            } else {
                const isPdf = item.name.endsWith('.pdf');
                const a = document.createElement('a');
                a.href = '#';
                a.className = 'file-link';
                a.innerHTML = `${isPdf ? this.icons.pdf : this.icons.markdown}<span class="name">${item.name}</span>`;
                a.onclick = (e) => {
                    e.preventDefault();
                    this.loadFile(item.path, isPdf, paneId);
                };
                li.appendChild(a);
            }

            parentUl.appendChild(li);
        }
    },

    // Load file content
    async loadFile(path, isPdf, paneId = 'primary') {
        const contentDiv = document.getElementById(paneId === 'primary' ? 'content-primary' : 'content-secondary');
        const headerDiv = contentDiv.querySelector('.content-header');
        const bodyDiv = contentDiv.querySelector('.content-body');

        // Update state
        if (paneId === 'primary') {
            this.state.currentFile = path;
        } else {
            this.state.compareFile = path;
        }

        // Update header
        headerDiv.innerHTML = `<span class="file-path">${path}</span>`;

        // Update URL hash
        this.updateHash();

        // Highlight active file in tree
        this.highlightActiveFile(path, paneId);

        if (isPdf) {
            bodyDiv.innerHTML = `<iframe src="${path}" class="pdf-viewer"></iframe>`;
        } else {
            try {
                bodyDiv.innerHTML = '<div class="loading">Loading...</div>';
                const response = await fetch(path);
                if (!response.ok) throw new Error('Failed to load file');
                const md = await response.text();
                bodyDiv.innerHTML = `<div class="markdown-body">${marked.parse(md)}</div>`;
            } catch (error) {
                console.error(error);
                bodyDiv.innerHTML = '<div class="error">Error loading content.</div>';
            }
        }
    },

    // Highlight active file in sidebar
    highlightActiveFile(path, paneId) {
        const treeId = paneId === 'primary' ? 'tree-primary' : 'tree-secondary';
        const tree = document.getElementById(treeId);
        if (!tree) return;

        tree.querySelectorAll('.file-link').forEach(link => {
            link.classList.remove('active');
        });

        tree.querySelectorAll('.file-link').forEach(link => {
            if (link.textContent.includes(path.split('/').pop())) {
                link.classList.add('active');
            }
        });
    },

    // Toggle compare mode
    toggleCompareMode() {
        this.state.compareMode = !this.state.compareMode;
        const viewer = document.getElementById('viewer');
        const btn = document.getElementById('compare-btn');

        if (this.state.compareMode) {
            viewer.classList.add('compare-mode');
            btn.classList.add('active');
            btn.innerHTML = `${this.icons.close}<span>Exit Compare</span>`;

            // Build secondary tree if not exists
            const secondaryTree = document.getElementById('tree-secondary');
            if (secondaryTree && secondaryTree.children.length === 0) {
                this.buildTree(this.fileTree, secondaryTree, 'secondary');
            }
        } else {
            viewer.classList.remove('compare-mode');
            btn.classList.remove('active');
            btn.innerHTML = `${this.icons.compare}<span>Compare</span>`;
            this.state.compareFile = null;
        }

        this.updateHash();
    },

    // Update URL hash for deep linking
    updateHash() {
        let hash = '';
        if (this.state.currentFile) {
            hash = this.state.currentFile;
            if (this.state.compareMode && this.state.compareFile) {
                hash += '|' + this.state.compareFile;
            }
        }
        if (hash) {
            window.location.hash = hash;
        }
    },

    // Parse URL hash
    parseHash() {
        const hash = window.location.hash.slice(1);
        if (!hash) return null;

        const parts = hash.split('|');
        return {
            primary: parts[0] || null,
            secondary: parts[1] || null,
        };
    },

    // Show intro content
    showIntro() {
        const contentDiv = document.getElementById('content-primary');
        const headerDiv = contentDiv.querySelector('.content-header');
        const bodyDiv = contentDiv.querySelector('.content-body');

        headerDiv.innerHTML = '<span class="file-path">Welcome</span>';
        bodyDiv.innerHTML = `
      <div class="intro">
        <h1>AI Model Assessment Experiment</h1>
        <p>
          This experiment evaluates how different AI models assess code repositories for 
          <strong>autonomous agent safety</strong>, <strong>enterprise-grade quality</strong>, 
          and <strong>implementation complexity</strong>.
        </p>
        <p>
          Browse the <strong>control groups</strong> (Dapr, Google Microservices Demo) and 
          <strong>experiment group</strong> (ODD Demonstration) assessments in the sidebar. 
          Use <strong>Compare mode</strong> to view two assessments side-by-side.
        </p>
        <div class="stats">
          <div class="stat">
            <span class="stat-value">3</span>
            <span class="stat-label">Repositories</span>
          </div>
          <div class="stat">
            <span class="stat-value">8+</span>
            <span class="stat-label">AI Models</span>
          </div>
          <div class="stat">
            <span class="stat-value">25</span>
            <span class="stat-label">Assessments</span>
          </div>
        </div>
      </div>
    `;
    },

    // Initialize application
    init() {
        // Build primary tree
        const primaryTree = document.getElementById('tree-primary');
        this.buildTree(this.fileTree, primaryTree, 'primary');

        // Setup compare button
        const compareBtn = document.getElementById('compare-btn');
        compareBtn.onclick = () => this.toggleCompareMode();

        // Check for hash on load
        const hashState = this.parseHash();
        if (hashState && hashState.primary) {
            const isPdf = hashState.primary.endsWith('.pdf');
            this.loadFile(hashState.primary, isPdf, 'primary');

            if (hashState.secondary) {
                this.toggleCompareMode();
                const isPdfSecondary = hashState.secondary.endsWith('.pdf');
                this.loadFile(hashState.secondary, isPdfSecondary, 'secondary');
            }
        } else {
            // Show intro by default
            this.showIntro();
        }

        // Handle hash changes
        window.onhashchange = () => {
            const state = this.parseHash();
            if (state && state.primary) {
                const isPdf = state.primary.endsWith('.pdf');
                this.loadFile(state.primary, isPdf, 'primary');
            }
        };
    }
};

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => ExperimentViewer.init());
