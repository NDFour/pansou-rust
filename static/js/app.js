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
  currentKeyword: '',
  currentFilters: {},
  activeMergeType: '__all__',

  // Pagination
  currentPage: 1,
  pageSize: 20,

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

    const config = { method, headers, ...opts };
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

  // Click stats
  async trackClick(payload) {
    return this.post('/stats/metric', payload, { keepalive: true });
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

function initSearchPage() {
  const searchInput = document.getElementById('search-input');
  const searchBtn = document.getElementById('search-btn');
  const filterBar = document.getElementById('filter-bar');

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

      // Pop animation
      chip.classList.add('pop');
      chip.addEventListener('animationend', () => chip.classList.remove('pop'), { once: true });

      if (state.currentKeyword) performSearch();
    });
  }
}

async function loadChannels() {
  const container = document.getElementById('channel-chips');
  if (!container) return;

  try {
    const health = await api.health();
    const channels = health.channels || [];

    container.innerHTML = channels.map((ch) => {
      const label = ch;
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
  state.currentPage = 1;
  const filters = getActiveFilters();

  // Build search params
  const params = {
    keyword,
    sourceType: filters.sourceType || 'all',
  };

  if (filters.channel) {
    params.channels = [filters.channel];
  }

  // Search button loading state
  const searchBtn = document.getElementById('search-btn');
  const searchBar = document.querySelector('.search-bar');
  if (searchBtn) searchBtn.classList.add('loading');
  if (searchBar) searchBar.classList.add('searching');

  // Show loading with animated skeletons + searching dots
  const resultsContainer = document.getElementById('results-container');
  if (resultsContainer) {
    resultsContainer.classList.remove('switching');
    resultsContainer.innerHTML = `
      <div style="text-align:center;padding:var(--space-2xl) 0;">
        <div class="spinner spinner-lg" style="margin:0 auto var(--space-lg);"></div>
        <p style="font-size:0.9375rem;color:var(--color-olive);">
          正在搜索
          <span class="searching-dots">
            <span></span><span></span><span></span>
          </span>
        </p>
      </div>
      <div class="skeleton-card skeleton skeleton-stagger skeleton-delay-1 mb-md"></div>
      <div class="skeleton-card skeleton skeleton-stagger skeleton-delay-2 mb-md"></div>
      <div class="skeleton-card skeleton skeleton-stagger skeleton-delay-3 mb-md"></div>
    `;
  }

  try {
    const data = await api.search(params);
    state.searchResults = data;
    // Brief delay for smooth transition
    if (resultsContainer) {
      resultsContainer.classList.add('switching');
      await new Promise(r => requestAnimationFrame(r));
    }
    renderResults();
  } catch (err) {
    const resultsContainer = document.getElementById('results-container');
    if (resultsContainer) {
      resultsContainer.innerHTML = `
        <div class="error-banner empty-state-entrance">
          <p>搜索失败：${escapeHtml(err.message)}</p>
        </div>
      `;
    }
  } finally {
    if (searchBtn) searchBtn.classList.remove('loading');
    if (searchBar) searchBar.classList.remove('searching');
  }
}

/* ============================================================
   Results Rendering
   ============================================================ */

function mergeByType(results) {
  const merged = {};
  const seen = new Set();

  results.forEach((result) => {
    (result.links || []).forEach((link) => {
      const key = link.url;
      if (seen.has(key)) return;
      seen.add(key);

      const type = link.disk_type || 'other';
      if (!merged[type]) merged[type] = [];

      merged[type].push({
        url: link.url,
        password: link.password || '',
        note: result.title || link.url,
        datetime: link.datetime || result.datetime,
        source: result.channel || 'unknown',
        images: result.images || [],
      });
    });
  });

  return merged;
}

function paginate(items, page, pageSize) {
  const start = (page - 1) * pageSize;
  const end = start + pageSize;
  return {
    pageItems: items.slice(start, end),
    totalPages: Math.max(1, Math.ceil(items.length / pageSize)),
    hasPrev: page > 1,
    hasNext: page < Math.ceil(items.length / pageSize),
  };
}

function renderPagination(page, totalPages, hasPrev, hasNext) {
  if (totalPages <= 1) return '';

  return `
    <div class="pagination">
      <button class="pagination-btn" data-action="prev" ${hasPrev ? '' : 'disabled'}>← 上一页</button>
      <span class="pagination-info">第 ${page}/${totalPages} 页</span>
      <button class="pagination-btn" data-action="next" ${hasNext ? '' : 'disabled'}>下一页 →</button>
    </div>`;
}

function renderResults() {
  const container = document.getElementById('results-container');
  if (!container) return;

  const data = state.searchResults;
  if (!data) {
    container.innerHTML = `
      <div class="empty-state empty-state-entrance">
        <div class="empty-state-icon animated">🔍</div>
        <h3>输入关键词开始搜索</h3>
      </div>
    `;
    return;
  }

  const total = data.total || 0;

  if (total === 0) {
    container.innerHTML = `
      <div class="empty-state empty-state-entrance">
        <div class="empty-state-icon animated">📭</div>
        <h3>未找到相关资源</h3>
      </div>
    `;
    return;
  }

  renderMergedResults(container, data);

  // Remove switching class for smooth transition
  container.classList.remove('switching');
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
  const merged = mergeByType(data.results || []);
  const types = Object.keys(merged);
  if (types.length === 0) {
    container.innerHTML = `
      <div class="empty-state empty-state-entrance">
        <div class="empty-state-icon animated">📭</div>
        <h3>未找到相关资源</h3>
      </div>`;
    return;
  }

  // Calculate total links
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

  // Collect visible links and paginate
  let visibleLinks = [];
  if (activeType === '__all__') {
    data.results.forEach((result) => {
      (result.links || []).forEach((link) => {
        const type = link.disk_type || 'other';
        visibleLinks.push({
          url: link.url,
          password: link.password || '',
          note: result.title || link.url,
          datetime: link.datetime || result.datetime,
          source: result.channel || 'unknown',
          images: result.images || [],
          _type: type,
        });
      });
    });
  } else {
    visibleLinks = (merged[activeType] || []).map((link) => ({ ...link, _type: activeType }));
  }

  const { pageItems, totalPages, hasPrev, hasNext } = paginate(visibleLinks, state.currentPage, state.pageSize);

  if (activeType === '__all__') {
    // Group paginated items by type for display
    const grouped = {};
    pageItems.forEach((link) => {
      const t = link._type;
      if (!grouped[t]) grouped[t] = [];
      grouped[t].push(link);
    });

    for (const type of Object.keys(grouped)) {
      const links = grouped[type];
      const label = TYPE_FRIENDLY[type] || type;
      html += `<div class="merged-group">
        <div class="merged-group-header">${escapeHtml(label)}</div>`;

      links.forEach((link, i) => {
        const delayClass = i <= 12 ? `card-delay-${i}` : 'card-delay-12';
        html += renderMergedCard(link, label, delayClass);
      });

      html += '</div>';
    }
  } else {
    if (pageItems.length > 0) {
      const label = TYPE_FRIENDLY[activeType] || activeType;
      html += `<div class="merged-group">
        <div class="merged-group-header">${escapeHtml(label)}</div>`;

      pageItems.forEach((link, i) => {
        const delayClass = i <= 12 ? `card-delay-${i}` : 'card-delay-12';
        html += renderMergedCard(link, label, delayClass);
      });

      html += '</div>';
    }
  }

  html += renderPagination(state.currentPage, totalPages, hasPrev, hasNext);

  container.innerHTML = html;
  bindResultEvents();
  bindMergeTabs();
  bindPaginationEvents();
}

function renderMergedCard(link, label, delayClass) {
  const channelLabel = link.source || '';
  return `
    <div class="merged-card merged-card-entrance ${delayClass}" data-url="${escapeHtml(link.url)}" data-password="${escapeHtml(link.password || '')}">
      <div class="result-card-header">
        <div>
          <div class="result-title" title="${escapeHtml(link.note || link.url)}">${escapeHtml(link.note || link.url)}</div>
          <a class="link-url mb-sm" href="${escapeHtml(link.url)}" target="_blank" rel="noopener noreferrer" style="display:block;">${escapeHtml(link.url)}</a>
          ${link.password ? `<div class="link-password">提取码: ${escapeHtml(link.password)} <button class="copy-btn" data-copy="${escapeHtml(link.password)}">复制</button></div>` : ''}
        </div>
        <div class="flex flex-col items-center gap-xs">
          <span class="tag">${escapeHtml(label)}</span>
          ${channelLabel ? `<span class="result-channel">${escapeHtml(channelLabel)}</span>` : ''}
          <span class="caption text-stone">${formatDate(link.datetime)}</span>
        </div>
      </div>
      ${link.images?.length ? `<div class="flex gap-sm mt-sm">${link.images.map(img => `<img src="${escapeHtml(img)}" alt="" style="width:60px;height:60px;object-fit:cover;border-radius:var(--radius-sm)" loading="lazy">`).join('')}</div>` : ''}
    </div>`;
}

function bindMergeTabs() {
  document.querySelectorAll('.type-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      state.activeMergeType = tab.dataset.mergeType;
      state.currentPage = 1;
      renderResults();
    });
  });
}

function bindPaginationEvents() {
  document.querySelectorAll('.pagination-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const action = btn.dataset.action;
      if (action === 'prev' && state.currentPage > 1) state.currentPage--;
      if (action === 'next') state.currentPage++;

      const container = document.getElementById('results-container');
      if (container) {
        container.classList.add('switching');
        requestAnimationFrame(() => { renderResults(); });
      }
    });
  });
}

function renderCheckAllBar(totalLinks) {
  return '';
  // 临时不启用，后续会使用
  // return `
  //   <div class="check-all-bar" id="check-all-bar">
  //     <span class="body-small text-olive">共 ${totalLinks} 个链接</span>
  //     <div class="flex items-center gap-md">
  //       <span id="check-progress" class="check-progress hidden"></span>
  //       <button class="btn btn-secondary btn-sm" id="check-all-btn" onclick="checkAllLinks()">
  //         🔗 检测全部链接状态
  //       </button>
  //     </div>
  //   </div>`;
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

function trackResultClick(title, url, channel) {
  const keyword = state.currentKeyword?.trim();
  if (!keyword || !title || !url || !channel) return;

  api.trackClick({ metric_type: 'click', metric_value: 1, keyword, title, url, channel }).catch(() => {
    // 统计不影响主流程，失败时静默忽略
  });
}

function bindResultEvents() {
  // Result link click stats
  document.querySelectorAll('.link-url').forEach((linkEl) => {
    linkEl.addEventListener('click', () => {
      const card = linkEl.closest('.merged-card');
      const titleEl = card?.querySelector('.result-title');
      const title = titleEl?.textContent?.trim() || linkEl.textContent?.trim() || '';
      const url = linkEl.getAttribute('href') || card?.dataset.url || '';
      const channel = card?.querySelector('.result-channel')?.textContent?.trim() || '';
      trackResultClick(title, url, channel);
    });
  });

  // Copy buttons
  document.querySelectorAll('.copy-btn').forEach((btn) => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const text = btn.dataset.copy;
      if (text) {
        copyToClipboard(text);
        btn.classList.add('copied', 'ripple');
        btn.textContent = '已复制 ✓';
        setTimeout(() => {
          btn.classList.remove('copied', 'ripple');
          if (btn.getAttribute('data-copy') && !btn.closest('.link-password')) {
            btn.textContent = '复制链接';
          } else {
            btn.textContent = '复制';
          }
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
