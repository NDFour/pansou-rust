/**
 * PanSou Frontend — API Client & UI Controller
 */

const API_BASE = '/api';

/* ============================================================
   State
   ============================================================ */
const state = {
  // Search state
  searchResults: null,
  viewMode: 'list', // 'list' | 'merged'
  currentKeyword: '',
  currentFilters: {},
  activeMergeType: '__all__',

  // Check state
  checkingAll: false,
};

/* ============================================================
   API Client
   ============================================================ */
const api = {
  async request(method, path, body = null, opts = {}) {
    const url = API_BASE + path;
    const headers = { 'Content-Type': 'application/json' };

    const config = { method, headers };
    if (body && method !== 'GET') {
      config.body = JSON.stringify(body);
    }

    const res = await fetch(url, config);

    const data = await res.json();
    if (!res.ok) {
      throw new Error(data.error || data.message || `HTTP ${res.status}`);
    }
    return data;
  },

  get(path, params = {}, opts = {}) {
    const qs = new URLSearchParams(params).toString();
    const fullPath = qs ? `${path}?${qs}` : path;
    return this.request('GET', fullPath, null, opts);
  },

  post(path, body = {}, opts = {}) {
    return this.request('POST', path, body, opts);
  },

  // Search
  async search(params) {
    const body = {
      kw: params.keyword || '',
      channels: params.channels || [],
      conc: params.concurrency || 0,
      refresh: params.forceRefresh || false,
      res: params.resultType || 'merged_by_type',
      src: params.sourceType || 'all',
      plugins: params.plugins || [],
      cloud_types: params.cloudTypes || [],
    };

    if (params.filter) {
      body.filter = params.filter;
    }

    const data = await this.post('/search', body);
    return data.data;
  },

  // Link checking
  async checkLinks(items) {
    const data = await this.post('/check/links', { items });
    return data;
  },

  // Health
  async health() {
    const data = await this.get('/health');
    return data;
  },
};

/* ============================================================
   Toast Notifications
   ============================================================ */
const Toast = {
  container: null,

  ensureContainer() {
    if (!this.container) {
      this.container = document.createElement('div');
      this.container.className = 'toast-container';
      document.body.appendChild(this.container);
    }
    return this.container;
  },

  show(message, type = 'info', duration = 4000) {
    const container = this.ensureContainer();
    const toast = document.createElement('div');
    toast.className = `toast toast-${type}`;
    toast.textContent = message;
    container.appendChild(toast);

    setTimeout(() => {
      toast.style.animation = 'toastOut 0.3s ease forwards';
      toast.addEventListener('animationend', () => toast.remove());
    }, duration);
  },

  success(msg) { this.show(msg, 'success'); },
  error(msg) { this.show(msg, 'error'); },
  info(msg) { this.show(msg, 'info'); },
};

/* ============================================================
   Search UI
   ============================================================ */

const CHANNEL_FRIENDLY = {
  tgsearchers6: 'TG搜索',
  yunpanpan: '云盘盘',
  zhao_source: '赵资源',
};

function initSearchPage() {
  const searchInput = document.getElementById('search-input');
  const searchBtn = document.getElementById('search-btn');
  const filterBar = document.getElementById('filter-bar');
  const viewTabs = document.querySelectorAll('.view-tab');

  // Search trigger
  if (searchBtn) {
    searchBtn.addEventListener('click', () => performSearch());
  }
  if (searchInput) {
    searchInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') performSearch();
    });
  }

  // Filter chips — event delegation
  if (filterBar) {
    filterBar.addEventListener('click', (e) => {
      const chip = e.target.closest('.filter-chip');
      if (!chip) return;
      const group = chip.dataset.filterGroup;
      if (!group) return;

      const siblings = filterBar.querySelectorAll(`.filter-chip[data-filter-group="${CSS.escape(group)}"]`);
      siblings.forEach((s) => s.classList.remove('active'));
      chip.classList.add('active');

      if (state.currentKeyword) performSearch();
    });
  }

  // View mode tabs
  viewTabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      viewTabs.forEach((t) => t.classList.remove('active'));
      tab.classList.add('active');
      state.viewMode = tab.dataset.view;
      renderResults();
    });
  });
}

async function loadChannels() {
  const container = document.getElementById('channel-chips');
  if (!container) return;

  try {
    const health = await api.health();
    const channels = health.channels || [];

    container.innerHTML = channels.map((ch) => {
      const label = CHANNEL_FRIENDLY[ch] || ch;
      return `<button class="filter-chip" data-filter-group="channel" data-filter-value="${escapeHtml(ch)}" title="${escapeHtml(ch)}">${escapeHtml(label)}</button>`;
    }).join('');
  } catch {
    // channels will fall back to config defaults
  }
}

function getActiveFilters() {
  const active = document.querySelectorAll('.filter-chip.active');
  const filters = { sourceType: 'all' };
  active.forEach((chip) => {
    const group = chip.dataset.filterGroup;
    const value = chip.dataset.filterValue;
    if (group && value) {
      filters[group] = value;
    }
  });
  return filters;
}

async function performSearch() {
  const input = document.getElementById('search-input');
  const keyword = input?.value?.trim();
  if (!keyword) {
    Toast.error('请输入搜索关键词');
    return;
  }

  state.currentKeyword = keyword;
  state.activeMergeType = '__all__';
  const filters = getActiveFilters();

  // Build search params
  const params = {
    keyword,
    sourceType: filters.sourceType || 'all',
    resultType: 'all',
  };

  if (filters.channel) {
    params.channels = [filters.channel];
  }

  // Show loading
  const resultsContainer = document.getElementById('results-container');
  if (resultsContainer) {
    resultsContainer.innerHTML = `
      <div class="skeleton-card skeleton mb-md"></div>
      <div class="skeleton-card skeleton mb-md"></div>
      <div class="skeleton-card skeleton mb-md"></div>
    `;
  }

  try {
    const data = await api.search(params);
    state.searchResults = data;
    renderResults();
  } catch (err) {
    const resultsContainer = document.getElementById('results-container');
    if (resultsContainer) {
      resultsContainer.innerHTML = `
        <div class="error-banner">
          <p>搜索失败：${escapeHtml(err.message)}</p>
        </div>
      `;
    }
  }
}

/* ============================================================
   Results Rendering
   ============================================================ */
function renderResults() {
  const container = document.getElementById('results-container');
  const countEl = document.getElementById('results-count');
  if (!container) return;

  const data = state.searchResults;
  if (!data) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-icon">🔍</div>
        <h3>输入关键词开始搜索</h3>
      </div>
    `;
    if (countEl) countEl.textContent = '';
    return;
  }

  const total = data.total || 0;
  if (countEl) {
    countEl.textContent = `共找到 ${total} 条结果`;
  }

  if (total === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-icon">📭</div>
        <h3>未找到相关资源</h3>
      </div>
    `;
    return;
  }

  if (state.viewMode === 'merged' && data.merged_by_type) {
    renderMergedResults(container, data);
  } else {
    renderListResults(container, data);
  }
}

function renderListResults(container, data) {
  const results = data.results || [];
  let html = '';

  results.forEach((result) => {
    html += renderResultCard(result);
  });

  // Check all bar if there are links
  const totalLinks = results.reduce((sum, r) => sum + (r.links?.length || 0), 0);
  if (totalLinks > 0) {
    html = renderCheckAllBar(totalLinks) + html;
  }

  container.innerHTML = html;
  bindResultEvents();
}

const TYPE_FRIENDLY = {
  baidu: '百度网盘',
  quark: '夸克网盘',
  aliyun: '阿里云盘',
  tianyi: '天翼云盘',
  xunlei: '迅雷云盘',
  '115': '115网盘',
  '123': '123云盘',
  uc: 'UC网盘',
  mobile: '移动云盘',
  magnet: '磁力链接',
  ed2k: '电驴链接',
};

function renderMergedResults(container, data) {
  const merged = data.merged_by_type || {};
  const types = Object.keys(merged);
  if (types.length === 0) {
    container.innerHTML = `
      <div class="empty-state">
        <div class="empty-state-icon">📭</div>
        <h3>未找到相关资源</h3>
      </div>`;
    return;
  }

  let totalLinks = 0;
  for (const links of Object.values(merged)) {
    totalLinks += links.length;
  }

  const activeType = state.activeMergeType || '__all__';

  // Tab bar
  let html = '<div class="type-tabs">';
  html += `<button class="type-tab ${activeType === '__all__' ? 'active' : ''}" data-merge-type="__all__">全部<span class="type-tab-count">${totalLinks}</span></button>`;
  for (const type of types) {
    const count = merged[type].length;
    const label = TYPE_FRIENDLY[type] || type;
    html += `<button class="type-tab ${activeType === type ? 'active' : ''}" data-merge-type="${escapeHtml(type)}">${escapeHtml(label)}<span class="type-tab-count">${count}</span></button>`;
  }
  html += '</div>';

  if (totalLinks > 0) {
    html += renderCheckAllBar(totalLinks);
  }

  // Render active type results
  const visibleTypes = activeType === '__all__' ? types : [activeType];
  for (const type of visibleTypes) {
    const links = merged[type];
    if (!links || links.length === 0) continue;
    const label = TYPE_FRIENDLY[type] || type;

    html += `<div class="merged-group">
      <div class="merged-group-header">${escapeHtml(label)} <span class="tag tag-coral">${links.length}</span></div>`;

    links.forEach((link) => {
      html += `
        <div class="merged-card" data-url="${escapeHtml(link.url)}" data-password="${escapeHtml(link.password || '')}">
          <div class="result-card-header">
            <div>
              <div class="result-title" title="${escapeHtml(link.note || link.url)}">${escapeHtml(link.note || link.url)}</div>
              <a class="link-url mb-sm" href="${escapeHtml(link.url)}" target="_blank" rel="noopener noreferrer" style="display:block;">${escapeHtml(link.url)}</a>
              ${link.password ? `<div class="link-password">提取码: ${escapeHtml(link.password)} <button class="copy-btn" data-copy="${escapeHtml(link.password)}">复制</button></div>` : ''}
            </div>
            <div class="flex flex-col items-center gap-xs">
              <span class="tag">${escapeHtml(label)}</span>
              <span class="caption text-stone">${formatDate(link.datetime)}</span>
            </div>
          </div>
          ${link.images?.length ? `<div class="flex gap-sm mt-sm">${link.images.map(img => `<img src="${escapeHtml(img)}" alt="" style="width:60px;height:60px;object-fit:cover;border-radius:var(--radius-sm)" loading="lazy">`).join('')}</div>` : ''}
        </div>`;
    });

    html += '</div>';
  }

  container.innerHTML = html;
  bindResultEvents();
  bindMergeTabs();
}

function bindMergeTabs() {
  document.querySelectorAll('.type-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      state.activeMergeType = tab.dataset.mergeType;
      renderResults();
    });
  });
}

function renderResultCard(result) {
  const links = result.links || [];
  const tags = result.tags || [];
  const images = result.images || [];

  let linksHtml = '';
  if (links.length > 0) {
    linksHtml = `
      <table class="links-table">
        <thead>
          <tr>
            <th>类型</th>
            <th>链接</th>
            <th>提取码</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          ${links
            .map(
              (link) => `
            <tr data-url="${escapeHtml(link.url)}" data-password="${escapeHtml(link.password || '')}">
              <td><span class="link-type-badge">${escapeHtml(link.disk_type)}</span></td>
              <td><a class="link-url" href="${escapeHtml(link.url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(link.url)}</a></td>
              <td>${link.password ? `<span class="link-password">${escapeHtml(link.password)} <button class="copy-btn" data-copy="${escapeHtml(link.password)}">复制</button></span>` : '<span class="text-stone">—</span>'}</td>
              <td><button class="btn btn-sm btn-secondary copy-btn" data-copy="${escapeHtml(link.url)}">复制链接</button></td>
            </tr>`
            )
            .join('')}
        </tbody>
      </table>`;
  }

  let tagsHtml = '';
  if (tags.length > 0) {
    tagsHtml =
      '<div class="flex flex-wrap gap-xs mt-md">' +
      tags.map((t) => `<span class="tag">${escapeHtml(t)}</span>`).join('') +
      '</div>';
  }

  let imagesHtml = '';
  if (images.length > 0) {
    imagesHtml =
      '<div class="flex gap-sm mt-md">' +
      images
        .map(
          (img) =>
            `<img src="${escapeHtml(img)}" alt="" style="width:80px;height:80px;object-fit:cover;border-radius:var(--radius-sm)" loading="lazy">`
        )
        .join('') +
      '</div>';
  }

  return `
    <div class="result-card">
      <div class="result-card-header">
        <div>
          <div class="result-title" title="${escapeHtml(result.title || '无标题')}">${escapeHtml(result.title || '无标题')}</div>
          <div class="result-meta">
            <span class="result-channel">${escapeHtml(result.channel)}</span>
            <span class="result-meta-item">${formatDate(result.datetime)}</span>
          </div>
        </div>
      </div>
      ${result.content ? `<div class="result-content">${escapeHtml(result.content)}</div>` : ''}
      ${linksHtml}
      ${imagesHtml}
      ${tagsHtml}
    </div>`;
}

function renderCheckAllBar(totalLinks) {
  return `
    <div class="check-all-bar" id="check-all-bar">
      <span class="body-small text-olive">共 ${totalLinks} 个链接</span>
      <div class="flex items-center gap-md">
        <span id="check-progress" class="check-progress hidden"></span>
        <button class="btn btn-secondary btn-sm" id="check-all-btn" onclick="checkAllLinks()">
          🔗 检测全部链接状态
        </button>
      </div>
    </div>`;
}

/* ============================================================
   Link Checking
   ============================================================ */
async function checkAllLinks() {
  if (state.checkingAll) return;
  state.checkingAll = true;

  const btn = document.getElementById('check-all-btn');
  const progressEl = document.getElementById('check-progress');
  if (btn) btn.disabled = true;

  // Collect all links
  const items = [];
  const rows = document.querySelectorAll('[data-url]');
  rows.forEach((row) => {
    const url = row.dataset.url;
    const password = row.dataset.password || '';
    if (url) {
      // Determine type from context
      let diskType = 'unknown';
      const typeBadge = row.querySelector('.link-type-badge');
      if (typeBadge) {
        diskType = typeBadge.textContent.trim();
      } else {
        // Try to infer from merged group header
        const group = row.closest('.merged-group');
        if (group) {
          const header = group.querySelector('.merged-group-header');
          if (header) diskType = header.childNodes[0]?.textContent?.trim() || 'unknown';
        }
      }
      items.push({ disk_type: diskType, url, password });
    }
  });

  if (items.length === 0) {
    Toast.info('没有可检测的链接');
    state.checkingAll = false;
    if (btn) btn.disabled = false;
    return;
  }

  if (progressEl) {
    progressEl.classList.remove('hidden');
    progressEl.innerHTML = '<span class="spinner"></span> 正在检测...';
  }

  try {
    const data = await api.checkLinks(items);
    const results = data.results || [];

    // Update link statuses
    const urlStatusMap = {};
    results.forEach((r) => {
      urlStatusMap[r.url] = r;
    });

    rows.forEach((row) => {
      const url = row.dataset.url;
      const status = urlStatusMap[url];
      if (status) {
        let statusHtml = '';
        if (status.state === 'valid' || status.state === 'accessible') {
          statusHtml = '<span class="link-status valid">● 有效</span>';
        } else if (status.state === 'invalid' || status.state === 'expired') {
          statusHtml = '<span class="link-status invalid">● 失效</span>';
        } else {
          statusHtml = '<span class="link-status unknown">● 未知</span>';
        }

        // Add status indicator to the row
        const existingStatus = row.querySelector('.link-status');
        if (existingStatus) {
          existingStatus.replaceWith(createElementFromHTML(statusHtml));
        } else {
          // Insert after last cell or append
          const lastCell = row.querySelector('td:last-child');
          if (lastCell) {
            lastCell.appendChild(createElementFromHTML(statusHtml));
          } else if (row.classList.contains('merged-card')) {
            const header = row.querySelector('.result-card-header > div:last-child');
            if (header) header.appendChild(createElementFromHTML(statusHtml));
          }
        }
      }
    });

    const validCount = results.filter((r) => r.state === 'valid' || r.state === 'accessible').length;
    if (progressEl) {
      progressEl.innerHTML = `<span style="color:var(--color-success)">● ${validCount}/${results.length} 有效</span>`;
    }
    Toast.success(`检测完成: ${validCount}/${results.length} 个链接有效`);
  } catch (err) {
    Toast.error(`检测失败: ${err.message}`);
    if (progressEl) progressEl.classList.add('hidden');
    if (btn) btn.disabled = false;
  } finally {
    state.checkingAll = false;
    if (btn) btn.disabled = false;
  }
}

/* ============================================================
   Utilities
   ============================================================ */
function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function formatDate(dateStr) {
  if (!dateStr) return '';
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return dateStr;
  }
}

function createElementFromHTML(html) {
  const template = document.createElement('template');
  template.innerHTML = html.trim();
  return template.content.firstChild;
}

function copyToClipboard(text) {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(text).then(() => {
      Toast.success('已复制到剪贴板');
    });
  } else {
    // Fallback
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.style.position = 'fixed';
    textarea.style.opacity = '0';
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    document.body.removeChild(textarea);
    Toast.success('已复制到剪贴板');
  }
}

function bindResultEvents() {
  // Copy buttons
  document.querySelectorAll('.copy-btn').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const text = btn.dataset.copy;
      if (text) {
        copyToClipboard(text);
        btn.classList.add('copied');
        btn.textContent = '已复制';
        setTimeout(() => {
          btn.classList.remove('copied');
          btn.textContent = '复制';
        }, 2000);
      }
    });
  });

  // Expand content
  document.querySelectorAll('.result-content').forEach((el) => {
    if (el.scrollHeight > el.clientHeight) {
      el.style.cursor = 'pointer';
      el.title = '点击展开/收起';
      el.addEventListener('click', () => {
        el.classList.toggle('expanded');
      });
    }
  });
}

/* ============================================================
   Init
   ============================================================ */
document.addEventListener('DOMContentLoaded', async () => {
  // Init search if on search page
  if (document.getElementById('search-input')) {
    initSearchPage();
    await loadChannels();
  }
});
