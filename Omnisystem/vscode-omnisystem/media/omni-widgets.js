/* ═══════════════════════════════════════════════════════════════════════════════
   Omni Widget Library — JavaScript Factory & Theme Engine
   OW namespace · Widget Database · Theme System · Enterprise-grade
   ═══════════════════════════════════════════════════════════════════════════════ */
'use strict';
(function(global) {

/* ── Theme registry ──────────────────────────────────────────────────────────── */
const OW_THEMES = [
  { id:'omni-dark',   label:'OmniDark',   swatch:'linear-gradient(135deg,#050D1A,#00D4FF)',  desc:'Deep space blue' },
  { id:'omni-light',  label:'OmniLight',  swatch:'linear-gradient(135deg,#EEF2FF,#1A6CF0)',  desc:'Clean & bright' },
  { id:'omni-neon',   label:'OmniNeon',   swatch:'linear-gradient(135deg,#000000,#00FF41)',  desc:'Terminal green' },
  { id:'omni-forest', label:'OmniForest', swatch:'linear-gradient(135deg,#050F07,#3CFF7E)',  desc:'Deep forest' },
  { id:'omni-aurora', label:'OmniAurora', swatch:'linear-gradient(135deg,#0B071E,#C07AFF)',  desc:'Violet aurora' },
  { id:'omni-sunset', label:'OmniSunset', swatch:'linear-gradient(135deg,#160700,#FF8C00)',  desc:'Warm sunset' },
];

/* ── Widget database — catalog of all widgets ─────────────────────────────────── */
const OW_WIDGET_DB = [
  // Buttons
  {id:'btn-primary',  cat:'Buttons',    label:'Primary Button',    desc:'Main call-to-action with accent color'},
  {id:'btn-solid',    cat:'Buttons',    label:'Solid Button',      desc:'High-contrast filled button'},
  {id:'btn-ghost',    cat:'Buttons',    label:'Ghost Button',      desc:'Borderless subtle button'},
  {id:'btn-danger',   cat:'Buttons',    label:'Danger Button',     desc:'Destructive action, red variant'},
  {id:'btn-success',  cat:'Buttons',    label:'Success Button',    desc:'Confirmation action, green'},
  {id:'btn-warning',  cat:'Buttons',    label:'Warning Button',    desc:'Caution action, gold/amber'},
  {id:'btn-icon',     cat:'Buttons',    label:'Icon Button',       desc:'Square icon-only button'},
  {id:'btn-fab',      cat:'Buttons',    label:'FAB',               desc:'Floating action button, circular'},
  {id:'btn-group',    cat:'Buttons',    label:'Button Group',      desc:'Segmented button cluster'},
  // Inputs
  {id:'input-text',   cat:'Inputs',     label:'Text Input',        desc:'Single-line text field'},
  {id:'input-search', cat:'Inputs',     label:'Search Input',      desc:'Search with icon and clear button'},
  {id:'input-textarea',cat:'Inputs',    label:'Textarea',          desc:'Multi-line text area'},
  {id:'input-select', cat:'Inputs',     label:'Select',            desc:'Dropdown selection'},
  {id:'input-toggle', cat:'Inputs',     label:'Toggle Switch',     desc:'On/off binary toggle'},
  {id:'input-checkbox',cat:'Inputs',    label:'Checkbox',          desc:'Multi-select checkbox'},
  {id:'input-radio',  cat:'Inputs',     label:'Radio Button',      desc:'Single-select radio'},
  {id:'input-slider', cat:'Inputs',     label:'Slider',            desc:'Range input with thumb'},
  // Cards
  {id:'card-basic',   cat:'Cards',      label:'Basic Card',        desc:'Container with border and shadow'},
  {id:'card-glass',   cat:'Cards',      label:'Glass Card',        desc:'Frosted glass with blur'},
  {id:'card-stat',    cat:'Cards',      label:'Stat Card',         desc:'KPI metric display with delta'},
  {id:'card-action',  cat:'Cards',      label:'Action Card',       desc:'Clickable card with icon + arrow'},
  {id:'card-flat',    cat:'Cards',      label:'Flat Card',         desc:'Borderless raised surface'},
  // Navigation
  {id:'nav-tabs',     cat:'Navigation', label:'Tabs',              desc:'Horizontal tab bar with active state'},
  {id:'nav-tabs-pill',cat:'Navigation', label:'Pill Tabs',         desc:'Rounded pill-style tab switcher'},
  {id:'nav-breadcrumb',cat:'Navigation',label:'Breadcrumbs',       desc:'Path navigation with separators'},
  {id:'nav-sidebar',  cat:'Navigation', label:'Sidebar Nav',       desc:'Vertical navigation with icons'},
  {id:'nav-toolbar',  cat:'Navigation', label:'Toolbar',           desc:'Horizontal action bar with groups'},
  {id:'nav-pagination',cat:'Navigation',label:'Pagination',        desc:'Page number navigation controls'},
  // Feedback
  {id:'fb-badge',     cat:'Feedback',   label:'Badge',             desc:'Status label with color variants'},
  {id:'fb-chip',      cat:'Feedback',   label:'Chip',              desc:'Dismissible tag with optional icon'},
  {id:'fb-progress',  cat:'Feedback',   label:'Progress Bar',      desc:'Animated fill with label support'},
  {id:'fb-spinner',   cat:'Feedback',   label:'Spinner',           desc:'Loading indicator, size variants'},
  {id:'fb-alert',     cat:'Feedback',   label:'Alert',             desc:'Info/success/warning/danger banner'},
  {id:'fb-toast',     cat:'Feedback',   label:'Toast',             desc:'Auto-dismiss notification'},
  {id:'fb-skeleton',  cat:'Feedback',   label:'Skeleton',          desc:'Content placeholder shimmer'},
  // Data
  {id:'data-table',   cat:'Data',       label:'Data Table',        desc:'Sortable rows with hover + select'},
  {id:'data-list',    cat:'Data',       label:'List',              desc:'Vertical item list with icons'},
  {id:'data-tree',    cat:'Data',       label:'Tree View',         desc:'Collapsible hierarchy'},
  {id:'data-code',    cat:'Data',       label:'Code Block',        desc:'Syntax container with copy button'},
  {id:'data-metric',  cat:'Data',       label:'Metric Display',    desc:'Large numeric KPI with trend'},
  {id:'data-sparkline',cat:'Data',      label:'Sparkline',         desc:'Mini bar chart for trends'},
  {id:'data-diff',    cat:'Data',       label:'Diff Viewer',       desc:'Add/remove/context line display'},
  // Overlays
  {id:'ol-modal',     cat:'Overlays',   label:'Modal',             desc:'Centered dialog with backdrop'},
  {id:'ol-drawer',    cat:'Overlays',   label:'Drawer',            desc:'Slide-in side panel'},
  {id:'ol-tooltip',   cat:'Overlays',   label:'Tooltip',           desc:'On-hover hint label'},
  {id:'ol-popover',   cat:'Overlays',   label:'Popover',           desc:'Anchored info panel'},
  {id:'ol-dropdown',  cat:'Overlays',   label:'Dropdown',          desc:'Contextual menu list'},
  {id:'ol-command',   cat:'Overlays',   label:'Command Palette',   desc:'Keyboard-driven search UI'},
  // Special
  {id:'sp-health',    cat:'Special',    label:'Health Ring',       desc:'Circular progress ring with value'},
  {id:'sp-terminal',  cat:'Special',    label:'Terminal',          desc:'Mock terminal with prompt'},
  {id:'sp-filetree',  cat:'Special',    label:'File Tree',         desc:'File/folder tree navigator'},
  {id:'sp-theme',     cat:'Special',    label:'Theme Picker',      desc:'Color swatch theme switcher'},
  {id:'sp-command',   cat:'Special',    label:'Command Palette',   desc:'Full command search overlay'},
  {id:'sp-wdb',       cat:'Special',    label:'Widget DB Card',    desc:'Preview card for widget catalog'},
];

/* ── Internal helpers ────────────────────────────────────────────────────────── */
function _el(tag, cls, attrs) {
  var e = document.createElement(tag);
  if (cls) e.className = cls;
  if (attrs) Object.entries(attrs).forEach(function(kv) {
    if (kv[0] === 'html') { e.innerHTML = kv[1]; }
    else if (kv[0] === 'text') { e.textContent = kv[1]; }
    else if (kv[0] === 'style') { e.style.cssText = kv[1]; }
    else { e.setAttribute(kv[0], kv[1]); }
  });
  return e;
}
function _append(parent) {
  var children = Array.prototype.slice.call(arguments, 1);
  children.forEach(function(c) { if (c) parent.appendChild(c); });
  return parent;
}
function _ripple(btn) {
  btn.addEventListener('click', function(e) {
    var r = _el('span','ow-ripple');
    var rect = btn.getBoundingClientRect();
    var size = Math.max(rect.width, rect.height);
    r.style.cssText = 'width:'+size+'px;height:'+size+'px;left:'+(e.clientX-rect.left-size/2)+'px;top:'+(e.clientY-rect.top-size/2)+'px';
    btn.appendChild(r);
    setTimeout(function() { r.remove(); }, 580);
  });
}

/* ── OW Public API ───────────────────────────────────────────────────────────── */
var OW = {

  /* Current theme */
  _theme: 'omni-dark',

  /* Widget Database */
  widgetDB: OW_WIDGET_DB,
  themes:   OW_THEMES,

  /* ── Theme system ──────────────────────────────────────────────────────────── */
  switchTheme: function(themeId) {
    if (!OW_THEMES.find(function(t){return t.id===themeId;})) return;
    var prev = OW._theme;
    OW._theme = themeId;
    document.documentElement.setAttribute('data-theme', themeId);
    // Smooth transition
    document.documentElement.classList.add('ow-theme-transition');
    setTimeout(function(){ document.documentElement.classList.remove('ow-theme-transition'); }, 350);
    // Update swatch active states
    document.querySelectorAll('.ow-theme-swatch').forEach(function(sw) {
      sw.classList.toggle('ow-active', sw.dataset.theme === themeId);
    });
    // Persist
    try { localStorage.setItem('ow-theme', themeId); } catch(e){}
    // Fire event
    document.dispatchEvent(new CustomEvent('ow-theme-change', {detail:{theme:themeId, prev:prev}}));
    // Notify extension host so it can broadcast to other panels
    if (OW._vscodeApi && !OW._owSyncing) {
      try { OW._vscodeApi.postMessage({command:'owThemeChange', theme:themeId}); } catch(e){}
    }
  },

  loadTheme: function() {
    var saved = null;
    try { saved = localStorage.getItem('ow-theme'); } catch(e){}
    OW.switchTheme(saved || 'omni-dark');
  },

  /* ── BUTTONS ───────────────────────────────────────────────────────────────── */
  btn: function(opts) {
    opts = opts || {};
    var b = _el('button', 'ow-btn' +
      (opts.variant ? ' ow-btn-'+opts.variant : ' ow-btn-primary') +
      (opts.size    ? ' ow-btn-'+opts.size    : '') +
      (opts.cls     ? ' '+opts.cls             : ''));
    if (opts.icon && opts.label) {
      _append(b, _el('span','',{text:opts.icon}), _el('span','ow-btn-label',{text:opts.label}));
    } else {
      b.textContent = opts.label || opts.text || 'Button';
    }
    if (opts.disabled) { b.disabled = true; b.setAttribute('aria-disabled','true'); }
    if (opts.title) b.title = opts.title;
    if (opts.onclick) b.addEventListener('click', opts.onclick);
    if (opts.ripple !== false) _ripple(b);
    return b;
  },

  btnGroup: function(buttons) {
    var g = _el('div','ow-btn-group');
    (buttons||[]).forEach(function(opts){ _append(g, OW.btn(opts)); });
    return g;
  },

  /* ── INPUTS ────────────────────────────────────────────────────────────────── */
  field: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-field');
    if (opts.label) {
      var lbl = _el('label','ow-label'+(opts.required?' ow-label-required':''),{text:opts.label});
      if (opts.id) lbl.setAttribute('for',opts.id);
      _append(wrap, lbl);
    }
    var input = OW.input(opts);
    _append(wrap, input);
    if (opts.hint)  _append(wrap, _el('div','ow-hint',{text:opts.hint}));
    if (opts.error) _append(wrap, _el('div','ow-errmsg',{html:'⚠ '+opts.error}));
    return wrap;
  },

  input: function(opts) {
    opts = opts || {};
    var i = _el('input','ow-input'+(opts.size?' ow-input-'+opts.size:'')+(opts.mono?' ow-input-mono':''));
    i.type = opts.type || 'text';
    if (opts.id)          i.id = opts.id;
    if (opts.placeholder) i.placeholder = opts.placeholder;
    if (opts.value)       i.value = opts.value;
    if (opts.readonly)    i.readOnly = true;
    if (opts.disabled)    i.disabled = true;
    if (opts.maxlength)   i.maxLength = opts.maxlength;
    if (opts.oninput)     i.addEventListener('input', opts.oninput);
    if (opts.onchange)    i.addEventListener('change', opts.onchange);
    if (opts.onenter)     i.addEventListener('keydown', function(e){ if(e.key==='Enter') opts.onenter(e); });
    return i;
  },

  searchInput: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-search-wrap');
    var icon = _el('span','ow-search-icon',{text:'🔍'});
    var inp  = OW.input(Object.assign({}, opts, {placeholder: opts.placeholder || 'Search…'}));
    var clear= _el('button','ow-clear',{text:'✕',type:'button','aria-label':'Clear'});
    clear.onclick = function(){ inp.value=''; if(opts.onchange) opts.onchange({target:inp}); inp.focus(); clear.style.display='none'; };
    inp.addEventListener('input', function(){ clear.style.display = this.value ? '' : 'none'; });
    clear.style.display = 'none';
    _append(wrap, icon, inp, clear);
    return wrap;
  },

  select: function(opts) {
    opts = opts || {};
    var s = _el('select','ow-select');
    if (opts.id) s.id = opts.id;
    if (opts.disabled) s.disabled = true;
    (opts.options||[]).forEach(function(o) {
      var op = _el('option','',{value:o.value||o,text:o.label||o.value||o});
      if (opts.value && (o.value||o) == opts.value) op.selected = true;
      _append(s, op);
    });
    if (opts.onchange) s.addEventListener('change', opts.onchange);
    return s;
  },

  toggle: function(opts) {
    opts = opts || {};
    var isOn = !!opts.on || !!opts.checked;
    var wrap = _el('div','ow-toggle-wrap');
    wrap.setAttribute('role','switch');
    wrap.setAttribute('aria-checked', String(isOn));
    wrap.setAttribute('tabindex','0');
    if (opts.label) wrap.setAttribute('aria-label', opts.label);
    var knob = _el('div','ow-toggle'+(opts.size?' ow-toggle-'+opts.size:'')+(isOn?' on':''));
    if (opts.id) { wrap.id = opts.id; }
    if (opts.label) _append(wrap, knob, _el('span','ow-toggle-label',{text:opts.label}));
    else _append(wrap, knob);
    var doToggle = function() {
      var on = knob.classList.toggle('on');
      wrap.setAttribute('aria-checked', String(on));
      if (opts.onchange) opts.onchange(on);
    };
    wrap.addEventListener('click', doToggle);
    wrap.addEventListener('keydown', function(e) {
      if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); doToggle(); }
    });
    wrap.toggle = function(state) { knob.classList.toggle('on', state); wrap.setAttribute('aria-checked', String(state)); };
    return wrap;
  },

  checkbox: function(opts) {
    opts = opts || {};
    var id = opts.id || ('ow-cb-'+Math.random().toString(36).slice(2));
    var wrap = _el('label','ow-cb-wrap');
    var cb = _el('input','ow-cb',{type:'checkbox',id:id});
    if (opts.checked) cb.checked = true;
    if (opts.disabled) cb.disabled = true;
    if (opts.onchange) cb.addEventListener('change', opts.onchange);
    _append(wrap, cb, _el('span','',{text:opts.label||''}));
    return wrap;
  },

  slider: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-slider-wrap');
    var inp = _el('input','ow-slider',{type:'range'});
    inp.min   = opts.min || 0;
    inp.max   = opts.max || 100;
    inp.value = opts.value || 50;
    inp.style.setProperty('--ow-pct', ((inp.value - inp.min)/(inp.max - inp.min)*100)+'%');
    inp.addEventListener('input', function() {
      this.style.setProperty('--ow-pct', ((this.value - this.min)/(this.max - this.min)*100)+'%');
      if (opts.oninput) opts.oninput(Number(this.value));
    });
    if (opts.labels) {
      _append(wrap, inp, _el('div','ow-slider-labels',{html:'<span>'+opts.labels[0]+'</span><span>'+opts.labels[1]+'</span>'}));
    } else {
      _append(wrap, inp);
    }
    return wrap;
  },

  /* ── CARDS ─────────────────────────────────────────────────────────────────── */
  card: function(opts) {
    opts = opts || {};
    var c = _el('div','ow-card'+(opts.variant?' ow-card-'+opts.variant:'')+(opts.interactive?' ow-card-interactive':'')+(opts.cls?' '+opts.cls:''));
    if (opts.title || opts.actions) {
      var hd = _el('div','ow-card-hd');
      if (opts.icon || opts.title) {
        var titleWrap = _el('div');
        if (opts.icon) _append(titleWrap, _el('span','',{text:opts.icon+' '}));
        _append(titleWrap, _el('span','ow-card-hd-title',{text:opts.title||''}));
        if (opts.subtitle) _append(titleWrap, _el('div','ow-card-hd-sub',{text:opts.subtitle}));
        _append(hd, titleWrap);
      }
      if (opts.actions) _append(hd, opts.actions);
      _append(c, hd);
    }
    if (opts.html) _append(c, _el('div','ow-card-body',{html:opts.html}));
    else if (opts.body) _append(c, opts.body instanceof HTMLElement ? opts.body : _el('div','ow-card-body',{text:opts.body}));
    if (opts.footer) {
      var ft = _el('div','ow-card-ft');
      _append(ft, opts.footer instanceof HTMLElement ? opts.footer : _el('span','',{text:opts.footer}));
      _append(c, ft);
    }
    if (opts.onclick) {
      c.style.cursor='pointer';
      c.setAttribute('tabindex','0');
      c.setAttribute('role','button');
      if (opts.ariaLabel) c.setAttribute('aria-label', opts.ariaLabel);
      c.addEventListener('click', opts.onclick);
      c.addEventListener('keydown', function(e){ if(e.key==='Enter'||e.key===' '){e.preventDefault();opts.onclick(e);} });
    }
    return c;
  },

  statCard: function(opts) {
    opts = opts || {};
    var c = _el('div','ow-stat'+(opts.cls?' '+opts.cls:''));
    c.style.setProperty('--ow-accent', opts.color||'var(--ow-accent)');
    _append(c, _el('div','ow-stat-val',{text:String(opts.value||0)}));
    if (opts.label) _append(c, _el('div','ow-stat-label',{text:opts.label}));
    if (opts.delta !== undefined) {
      var dv = Number(opts.delta);
      var dcls = dv>0?'ow-stat-up':dv<0?'ow-stat-dn':'ow-stat-flat';
      _append(c, _el('div','ow-stat-sub '+dcls,{text:(dv>0?'▲ +':dv<0?'▼ ':'→ ')+Math.abs(dv)+(opts.deltaUnit||'%')}));
    }
    if (opts.bar !== undefined) {
      var bar = _el('div','ow-stat-bar');
      var fill = _el('div','ow-stat-bar-fill');
      fill.style.width = Math.min(100,Math.max(0,opts.bar))+'%';
      _append(bar, fill); _append(c, bar);
    }
    return c;
  },

  actionCard: function(opts) {
    opts = opts || {};
    var c = _el('button','ow-action'+(opts.cls?' '+opts.cls:''));
    var icon = _el('div','ow-action-icon',{text:opts.icon||'✦'});
    var body = _el('div');
    _append(body, _el('div','ow-action-title',{text:opts.title||''}));
    if (opts.desc) _append(body, _el('div','ow-action-desc',{text:opts.desc}));
    _append(c, icon, body, _el('span','ow-action-arrow',{text:'›'}));
    if (opts.onclick) c.addEventListener('click', opts.onclick);
    _ripple(c);
    return c;
  },

  /* ── NAVIGATION ────────────────────────────────────────────────────────────── */
  tabs: function(opts) {
    opts = opts || {};
    var wrap = _el('div');
    var bar  = _el('div','ow-tabs'+(opts.pill?' ow-tabs-pill':''),{'role':'tablist'});
    var panels = _el('div','ow-tab-panels');
    var tabEls = [], panelEls = [];

    (opts.tabs||[]).forEach(function(tab, i) {
      var panelId = 'ow-panel-'+i+'-'+Math.random().toString(36).slice(2);
      var tabId   = 'ow-tab-'+i+'-'+Math.random().toString(36).slice(2);
      var t = _el('button','ow-tab'+(opts.pill?' ow-tab-pill':'')+(i===0?' ow-active':''));
      t.setAttribute('role','tab');
      t.setAttribute('aria-selected', i===0 ? 'true' : 'false');
      t.setAttribute('aria-controls', panelId);
      t.id = tabId;
      t.innerHTML = tab.label + (tab.count!==undefined ? '<span class="ow-tab-count">'+tab.count+'</span>' : '');
      t.addEventListener('click', function() {
        tabEls.forEach(function(x,j){
          x.classList.remove('ow-active');
          x.setAttribute('aria-selected','false');
          panelEls[j].style.display='none';
        });
        t.classList.add('ow-active');
        t.setAttribute('aria-selected','true');
        panelEls[i].style.display='';
        if (opts.onchange) opts.onchange(i, tab);
      });
      t.addEventListener('keydown', function(e) {
        if (e.key === 'ArrowRight') { e.preventDefault(); tabEls[Math.min(tabEls.length-1, i+1)].click(); tabEls[Math.min(tabEls.length-1, i+1)].focus(); }
        if (e.key === 'ArrowLeft')  { e.preventDefault(); tabEls[Math.max(0, i-1)].click(); tabEls[Math.max(0, i-1)].focus(); }
      });
      _append(bar, t);
      tabEls.push(t);
      var p = _el('div','ow-tab-panel'+(i!==0?' ow-hidden':''));
      p.id = panelId;
      p.setAttribute('role','tabpanel');
      p.setAttribute('aria-labelledby', tabId);
      if (i!==0) p.style.display='none';
      if (tab.content instanceof HTMLElement) _append(p, tab.content);
      else if (tab.html) p.innerHTML = tab.html;
      _append(panels, p);
      panelEls.push(p);
    });
    _append(wrap, bar, panels);
    return wrap;
  },

  breadcrumbs: function(items) {
    var nav = _el('nav','ow-crumbs',{'aria-label':'breadcrumbs'});
    (items||[]).forEach(function(item, i) {
      if (i>0) _append(nav, _el('span','ow-crumb-sep',{text:'/'}));
      var crumb = _el('span','ow-crumb'+(i===items.length-1?' ow-active':''));
      if (item.href || item.onclick) {
        var a = _el('button','ow-crumb-link',{text:item.label||item});
        if (item.onclick) a.addEventListener('click', item.onclick);
        _append(crumb, a);
      } else {
        crumb.textContent = item.label || item;
      }
      _append(nav, crumb);
    });
    return nav;
  },

  sideNav: function(opts) {
    opts = opts || {};
    var nav = _el('nav','ow-sidenav',{'aria-label':opts.label||'Navigation'});
    (opts.sections||[{items:opts.items||[]}]).forEach(function(section) {
      if (section.label) _append(nav, _el('div','ow-sidenav-hd',{text:section.label}));
      if (section.sep)   _append(nav, _el('div','ow-sidenav-sep'));
      (section.items||[]).forEach(function(item) {
        var li = _el('div','ow-sidenav-item'+(item.active?' ow-active':''),{'role':'button','tabindex':'0'});
        if (item.icon) _append(li, _el('span','ow-sidenav-icon',{text:item.icon}));
        _append(li, _el('span','ow-sidenav-label',{text:item.label||''}));
        if (item.badge) _append(li, _el('span','ow-sidenav-badge',{text:item.badge}));
        if (item.onclick) li.addEventListener('click', item.onclick);
        li.addEventListener('keydown', function(e){ if(e.key==='Enter'||e.key===' ') { e.preventDefault(); li.click(); } });
        _append(nav, li);
      });
    });
    return nav;
  },

  pagination: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-pages',{'aria-label':'Pagination'});
    var total = opts.total||1, current = opts.page||1;
    var prev = _el('button','ow-page-btn',{text:'‹','aria-label':'Previous'});
    prev.disabled = current<=1;
    prev.addEventListener('click', function(){ if(opts.onpage) opts.onpage(current-1); });
    _append(wrap, prev);
    var pages = OW._pageList(current, total);
    pages.forEach(function(p) {
      if (p==='...') { _append(wrap, _el('span','ow-page-ellipsis',{text:'…'})); return; }
      var btn = _el('button','ow-page-btn'+(p===current?' ow-active':''),{text:String(p)});
      btn.addEventListener('click', function(){ if(opts.onpage) opts.onpage(p); });
      _append(wrap, btn);
    });
    var next = _el('button','ow-page-btn',{text:'›','aria-label':'Next'});
    next.disabled = current>=total;
    next.addEventListener('click', function(){ if(opts.onpage) opts.onpage(current+1); });
    _append(wrap, next);
    return wrap;
  },

  _pageList: function(current, total) {
    if (total<=7) return Array.from({length:total},function(_,i){return i+1;});
    var pages = [];
    if (current<=4) { pages=[1,2,3,4,5,'...',total]; }
    else if (current>=total-3) { pages=[1,'...',total-4,total-3,total-2,total-1,total]; }
    else { pages=[1,'...',current-1,current,current+1,'...',total]; }
    return pages;
  },

  /* ── FEEDBACK ──────────────────────────────────────────────────────────────── */
  badge: function(text, variant, pulse) {
    var b = _el('span','ow-badge'+(variant?' ow-badge-'+variant:'')+(pulse?' ow-badge-pulse':''));
    if (pulse) _append(b, _el('span','ow-badge-dot'), _el('span','',{text:' '+text}));
    else b.textContent = text;
    return b;
  },

  chip: function(opts) {
    opts = typeof opts==='string' ? {label:opts} : (opts||{});
    var c = _el('span','ow-chip'+(opts.active?' ow-active':'')+(opts.cls?' '+opts.cls:''));
    if (opts.icon) _append(c, _el('span','ow-chip-icon',{text:opts.icon}));
    _append(c, _el('span','',{text:opts.label||''}));
    if (opts.onclose) {
      var x = _el('button','ow-chip-close',{text:'✕','aria-label':'Remove'});
      x.addEventListener('click', function(e){ e.stopPropagation(); opts.onclose(c); });
      _append(c, x);
    }
    if (opts.onclick) c.addEventListener('click', opts.onclick);
    return c;
  },

  progress: function(opts) {
    if (typeof opts==='number') opts={value:opts};
    opts = opts||{};
    var wrap = _el('div');
    if (opts.label || opts.showPct) {
      var hd = _el('div','ow-progress-hd');
      if (opts.label) _append(hd, _el('span','',{text:opts.label}));
      if (opts.showPct) _append(hd, _el('span','',{text:Math.round(opts.value||0)+'%'}));
      _append(wrap, hd);
    }
    var bar  = _el('div','ow-progress'+(opts.size?' ow-progress-'+opts.size:'')+(opts.striped?' ow-progress-strip':''));
    var fill = _el('div','ow-progress-fill'+(opts.variant?' ow-'+opts.variant:''));
    fill.style.width = Math.min(100,Math.max(0,opts.value||0))+'%';
    if (opts.color) fill.style.background = opts.color;
    fill.setAttribute('role','progressbar');
    fill.setAttribute('aria-valuenow', String(opts.value||0));
    fill.setAttribute('aria-valuemin','0');
    fill.setAttribute('aria-valuemax','100');
    _append(bar, fill); _append(wrap, bar);
    wrap.setValue = function(v) { fill.style.width=Math.min(100,Math.max(0,v))+'%'; fill.setAttribute('aria-valuenow',String(v)); };
    return wrap;
  },

  spinner: function(size) {
    return _el('div','ow-spin'+(size?' ow-spin-'+size:''),{'role':'status','aria-label':'Loading'});
  },

  alert: function(opts) {
    opts = typeof opts==='string' ? {msg:opts} : (opts||{});
    var a = _el('div','ow-alert'+(opts.variant?' ow-alert-'+opts.variant:''),{'role':'alert'});
    if (opts.icon) _append(a, _el('div','ow-alert-icon',{text:opts.icon}));
    var body = _el('div','ow-alert-body');
    if (opts.title) _append(body, _el('div','ow-alert-title',{text:opts.title}));
    _append(body, _el('div','ow-alert-msg',{text:opts.msg||''}));
    _append(a, body);
    if (opts.closable !== false) {
      var x = _el('button','ow-alert-close',{text:'✕','aria-label':'Dismiss'});
      x.addEventListener('click', function(){ a.style.animation='ow-fade-out 0.2s forwards'; setTimeout(function(){a.remove();},220); });
      _append(a, x);
    }
    return a;
  },

  toast: function(opts) {
    opts = typeof opts==='string' ? {msg:opts} : (opts||{});
    var icons = {success:'✅',warning:'⚠️',danger:'❌',info:'💡'};
    var t = _el('div','ow-toast');
    _append(t, _el('span','ow-toast-icon',{text:opts.icon||icons[opts.variant]||'💡'}));
    var body = _el('div','ow-toast-body');
    if (opts.title) _append(body, _el('div','ow-toast-title',{text:opts.title}));
    _append(body, _el('div','ow-toast-msg',{text:opts.msg||''}));
    _append(t, body);
    var x = _el('button','ow-toast-close',{text:'✕','aria-label':'Dismiss'});
    x.addEventListener('click', function(){ OW._removeToast(t); });
    _append(t, x);
    OW._getToastContainer().prepend(t);
    if (opts.duration !== 0) {
      setTimeout(function(){ OW._removeToast(t); }, opts.duration||4000);
    }
    return t;
  },

  _removeToast: function(t) {
    t.classList.add('ow-out');
    setTimeout(function(){t.remove();}, 300);
  },

  _getToastContainer: function() {
    var c = document.getElementById('ow-toast-container');
    if (!c) {
      c = _el('div','',{id:'ow-toast-container',
        'role':'region','aria-label':'Notifications','aria-live':'polite','aria-atomic':'false',
        style:'position:fixed;bottom:16px;right:16px;z-index:9999;display:flex;flex-direction:column;gap:8px;max-width:320px'});
      document.body.appendChild(c);
    }
    return c;
  },

  skeleton: function(opts) {
    opts = opts || {};
    var wrap = _el('div');
    if (opts.type === 'card') {
      _append(wrap, _el('div','ow-skel ow-skel-card'));
    } else if (opts.type === 'list') {
      for (var i=0;i<(opts.rows||3);i++) {
        var row = _el('div','ow-row ow-gap-2',{style:'padding:6px 0'});
        _append(row, _el('div','ow-skel ow-skel-avatar'), _el('div','',{style:'flex:1'}));
        _append(row.lastChild, _el('div','ow-skel ow-skel-title'), _el('div','ow-skel ow-skel-text'));
        _append(wrap, row);
      }
    } else {
      _append(wrap, _el('div','ow-skel ow-skel-title'));
      for (var j=0;j<(opts.rows||3);j++) _append(wrap, _el('div','ow-skel ow-skel-text',{style:'width:'+(95-j*8)+'%'}));
    }
    return wrap;
  },

  /* ── DATA DISPLAY ──────────────────────────────────────────────────────────── */
  table: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-tbl-wrap ow-scroll');
    var tbl  = _el('table','ow-tbl'+(opts.compact?' ow-tbl-compact':'')+(opts.mono?' ow-tbl-mono':''));
    // Header
    var thead = _el('thead');
    var tr = _el('tr');
    (opts.cols||[]).forEach(function(col) {
      var th = _el('th',col.cls||'',{text:col.label||col.key||col});
      if (opts.sortable && col.key) {
        th.style.cursor='pointer';
        th.addEventListener('click', function(){
          if (opts.onsort) opts.onsort(col.key);
        });
      }
      _append(tr, th);
    });
    _append(thead, tr); _append(tbl, thead);
    // Body
    var tbody = _el('tbody');
    (opts.rows||[]).forEach(function(row, ri) {
      var tr2 = _el('tr','');
      (opts.cols||[]).forEach(function(col) {
        var key = col.key || col;
        var td = _el('td','');
        var val = row[key];
        if (col.render) td.innerHTML = col.render(val, row, ri);
        else td.textContent = val !== undefined ? String(val) : '';
        _append(tr2, td);
      });
      if (opts.onrow) tr2.addEventListener('click', function(){ opts.onrow(row, ri); });
      _append(tbody, tr2);
    });
    _append(tbl, tbody);
    _append(wrap, tbl);
    if (opts.empty && !(opts.rows||[]).length) {
      _append(wrap, OW.empty({icon:'🗃',title:'No data',desc:opts.empty}));
    }
    return wrap;
  },

  code: function(opts) {
    opts = typeof opts==='string' ? {code:opts} : (opts||{});
    var box = _el('div','ow-code');
    var hd  = _el('div','ow-code-hd');
    var dots= _el('div','ow-code-dots');
    ['#FF5F57','#FFBD2E','#28C840'].forEach(function(c){
      var d = _el('div','ow-code-dot'); d.style.background=c; _append(dots,d);
    });
    _append(hd, dots);
    if (opts.lang) _append(hd, _el('span','ow-code-lang',{text:opts.lang.toUpperCase()}));
    var copyBtn = _el('button','ow-code-copy',{text:'Copy','aria-label':'Copy code'});
    copyBtn.addEventListener('click', function(){
      try{ navigator.clipboard.writeText(opts.code||''); copyBtn.textContent='✓ Copied'; setTimeout(function(){copyBtn.textContent='Copy';},1500); }catch(e){}
    });
    _append(hd, copyBtn);
    var body = _el('div','ow-code-body ow-scroll');
    body.textContent = opts.code || '';
    _append(box, hd, body);
    return box;
  },

  metric: function(opts) {
    opts = opts || {};
    var w = _el('div','ow-metric');
    var valEl = _el('div','ow-metric-val');
    valEl.innerHTML = opts.value + (opts.unit ? '<span class="ow-metric-unit">'+opts.unit+'</span>' : '');
    if (opts.color) valEl.style.color = opts.color;
    _append(w, valEl);
    if (opts.label) _append(w, _el('div','ow-metric-lbl',{text:opts.label}));
    if (opts.trend !== undefined) {
      var sign = opts.trend>0?'▲ +':opts.trend<0?'▼ ':'→ ';
      var col  = opts.trend>0?'var(--ow-success)':opts.trend<0?'var(--ow-danger)':'var(--ow-text-muted)';
      var tr = _el('div','ow-metric-trend',{text:sign+Math.abs(opts.trend)+(opts.trendUnit||'%')});
      tr.style.color = col; _append(w, tr);
    }
    return w;
  },

  sparkline: function(values, opts) {
    opts = opts || {};
    var max = Math.max.apply(null, values) || 1;
    var wrap = _el('div','ow-sparkline',{'aria-label':'Trend chart','role':'img'});
    values.forEach(function(v) {
      var bar = _el('div','ow-spark-bar');
      bar.style.height = Math.max(4, (v/max)*24)+'px';
      if (opts.color) bar.style.background = opts.color;
      _append(wrap, bar);
    });
    return wrap;
  },

  /* ── OVERLAYS ──────────────────────────────────────────────────────────────── */
  modal: function(opts) {
    opts = opts || {};
    var bg = _el('div','ow-modal-bg'+(opts.size?' ow-modal-'+opts.size:''),{'role':'dialog','aria-modal':'true','aria-labelledby':'ow-modal-title'});
    var m  = _el('div','ow-modal');
    var hd = _el('div','ow-modal-hd');
    _append(hd, _el('div','ow-modal-title',{id:'ow-modal-title',text:opts.title||''}));
    var closeBtn = _el('button','ow-modal-close',{text:'✕','aria-label':'Close'});
    var closeModal = function(){ bg.style.animation='ow-fade-out 0.15s forwards'; setTimeout(function(){bg.remove();if(opts.onclose)opts.onclose();},160); };
    closeBtn.addEventListener('click', closeModal);
    bg.addEventListener('click', function(e){ if(e.target===bg && opts.closeOnBack!==false) closeModal(); });
    bg.addEventListener('keydown', function(e){ if(e.key==='Escape' && opts.closeOnEsc!==false) closeModal(); });
    _append(hd, closeBtn);
    _append(m, hd);
    if (opts.body instanceof HTMLElement) _append(m, _el('div','ow-modal-body',{}).appendChild(opts.body));
    else _append(m, _el('div','ow-modal-body',{html:opts.body||opts.html||''}));
    if (opts.footer) {
      var ft = _el('div','ow-modal-ft');
      _append(ft, opts.footer instanceof HTMLElement ? opts.footer : _el('div','',{html:opts.footer}));
      _append(m, ft);
    } else if (opts.buttons) {
      var ft2 = _el('div','ow-modal-ft');
      (opts.buttons||[]).forEach(function(b){ _append(ft2, OW.btn(b)); });
      _append(m, ft2);
    }
    _append(bg, m);
    document.body.appendChild(bg);
    // Trap focus
    setTimeout(function(){ m.querySelector('button,input,select,textarea,[tabindex]')?.focus(); }, 50);
    bg.close = closeModal;
    return bg;
  },

  dropdown: function(opts) {
    opts = opts || {};
    var wrap = _el('div','ow-dd-wrap');
    var trigger = opts.trigger || OW.btn(Object.assign({},opts.btnOpts||{},{label:opts.label||'Menu',variant:'ghost'}));
    _append(wrap, trigger);
    var menu = null;
    var close = function() {
      if (menu) { menu.style.animation='ow-fade-out 0.1s forwards'; setTimeout(function(){if(menu)menu.remove();menu=null;},110); }
    };
    trigger.addEventListener('click', function(e) {
      e.stopPropagation();
      if (menu) { close(); return; }
      menu = _el('div','ow-dd-menu');
      if (opts.title) _append(menu, _el('div','ow-cmd-section',{text:opts.title}));
      (opts.items||[]).forEach(function(item) {
        if (item.sep) { _append(menu, _el('div','ow-dd-sep')); return; }
        var li = _el('div','ow-dd-item'+(item.cls?' '+item.cls:'')+(item.active?' ow-active':'')+(item.danger?' ow-danger':''));
        if (item.icon) _append(li, _el('span','ow-dd-icon',{text:item.icon}));
        _append(li, _el('span','',{text:item.label||''}));
        if (item.key) _append(li, _el('span','ow-cmd-result-key',{text:item.key}));
        li.addEventListener('click', function(e){ e.stopPropagation(); close(); if(item.onclick) item.onclick(); });
        _append(menu, li);
      });
      _append(wrap, menu);
    });
    document.addEventListener('click', close);
    return wrap;
  },

  /* ── SPECIAL ───────────────────────────────────────────────────────────────── */
  healthRing: function(value, opts) {
    opts = opts || {};
    var size = opts.size || 80;
    var strokeW = opts.strokeW || 7;
    var r = (size/2) - strokeW;
    var circ = 2 * Math.PI * r;
    var pct = Math.min(100,Math.max(0,value));
    var offset = circ - (pct/100)*circ;
    var color = opts.color || (pct>80?'var(--ow-success)':pct>50?'var(--ow-warning)':'var(--ow-danger)');
    var wrap = _el('div','ow-ring');
    wrap.innerHTML = '<svg width="'+size+'" height="'+size+'" viewBox="0 0 '+size+' '+size+'" aria-label="'+Math.round(pct)+'%">'
      +'<circle class="ow-ring-track" cx="'+size/2+'" cy="'+size/2+'" r="'+r+'" stroke-width="'+strokeW+'"/>'
      +'<circle class="ow-ring-fill" cx="'+size/2+'" cy="'+size/2+'" r="'+r+'" stroke-width="'+strokeW+'"'
      +' stroke-dasharray="'+circ+'" stroke-dashoffset="'+offset+'" stroke="'+color+'"/>'
      +'</svg>';
    var val = _el('div','ow-ring-val',{text:Math.round(pct)+(opts.unit||'%')});
    val.style.fontSize = (size*0.22)+'px';
    val.style.color = color;
    _append(wrap, val);
    wrap.setValue = function(v) {
      var p2=Math.min(100,Math.max(0,v));
      var o2=circ-(p2/100)*circ;
      var c2=opts.color||(p2>80?'var(--ow-success)':p2>50?'var(--ow-warning)':'var(--ow-danger)');
      wrap.querySelector('.ow-ring-fill').setAttribute('stroke-dashoffset',o2);
      wrap.querySelector('.ow-ring-fill').setAttribute('stroke',c2);
      val.textContent=Math.round(p2)+(opts.unit||'%');
      val.style.color=c2;
    };
    return wrap;
  },

  themePicker: function(opts) {
    opts = opts || {};
    var wrap = _el('div');
    _append(wrap, _el('div','ow-section-lbl',{text:'Theme'}));
    var swatches = _el('div','ow-themes');
    OW_THEMES.forEach(function(th) {
      var tile = _el('div','ow-theme-tile');
      var sw = _el('div','ow-theme-swatch'+(th.id===OW._theme?' ow-active':''));
      sw.dataset.theme = th.id;
      sw.style.background = th.swatch;
      sw.title = th.label+' — '+th.desc;
      sw.setAttribute('role','radio');
      sw.setAttribute('aria-checked', String(th.id===OW._theme));
      sw.setAttribute('aria-label', th.label);
      sw.addEventListener('click', function(){
        OW.switchTheme(th.id);
        if (opts.onchange) opts.onchange(th.id);
      });
      _append(tile, sw, _el('div','ow-theme-label',{text:th.label}));
      _append(swatches, tile);
    });
    _append(wrap, swatches);
    return wrap;
  },

  empty: function(opts) {
    opts = typeof opts==='string' ? {title:opts} : (opts||{});
    var w = _el('div','ow-empty');
    if (opts.icon) _append(w, _el('div','ow-empty-icon',{text:opts.icon}));
    if (opts.title) _append(w, _el('div','ow-empty-title',{text:opts.title}));
    if (opts.desc) _append(w, _el('div','ow-empty-desc',{text:opts.desc}));
    if (opts.action) _append(w, OW.btn(opts.action));
    return w;
  },

  kbd: function(keys) {
    if (typeof keys==='string') keys = keys.split('+');
    var wrap = _el('span',{style:'display:inline-flex;align-items:center;gap:3px'});
    keys.forEach(function(k,i){
      if (i>0) _append(wrap, _el('span','',{text:'+',style:'color:var(--ow-text-muted);font-size:9px'}));
      _append(wrap, _el('kbd','ow-kbd',{text:k}));
    });
    return wrap;
  },

  /* ── Widget Database Browser ────────────────────────────────────────────────── */
  widgetBrowser: function(opts) {
    opts = opts || {};
    var categories = [...new Set(OW_WIDGET_DB.map(function(w){return w.cat;}))];
    var activecat = 'All';
    var wrap = _el('div','ow-stack ow-gap-3');
    var search = OW.searchInput({placeholder:'Search widgets…'});
    var tabs   = _el('div','ow-tabs-pill',{style:'display:flex;gap:4px;flex-wrap:wrap'});
    var grid   = _el('div','ow-grid-auto ow-gap-2',{style:'margin-top:8px'});

    var render = function() {
      var q = search.querySelector('input').value.toLowerCase();
      grid.innerHTML = '';
      OW_WIDGET_DB.filter(function(w){
        var catMatch = activecat==='All'||w.cat===activecat;
        var srchMatch= !q||(w.label+' '+w.desc+' '+w.cat).toLowerCase().includes(q);
        return catMatch&&srchMatch;
      }).forEach(function(w) {
        var c = _el('div','ow-wdb-card');
        var prev = _el('div','ow-wdb-preview');
        prev.innerHTML = OW._widgetPreview(w.id);
        var meta = _el('div','ow-wdb-meta');
        _append(meta, _el('div','ow-wdb-name',{text:w.label}), _el('div','ow-wdb-cat',{text:w.cat}), _el('div','ow-wdb-desc',{text:w.desc}));
        _append(c, prev, meta);
        if (opts.onselect) c.addEventListener('click', function(){ opts.onselect(w); });
        _append(grid, c);
      });
      if (!grid.children.length) _append(grid, OW.empty({icon:'🔍',title:'No widgets found',desc:'Try a different search term'}));
    };

    // Category buttons
    _append(tabs, OW.chip({label:'All '+OW_WIDGET_DB.length, active:true, onclick:function(){activecat='All';tabs.querySelectorAll('.ow-chip').forEach(function(c){c.classList.remove('ow-active')});this.classList.add('ow-active');render();}}));
    categories.forEach(function(cat){
      var count = OW_WIDGET_DB.filter(function(w){return w.cat===cat;}).length;
      _append(tabs, OW.chip({label:cat+' '+count, onclick:function(){activecat=cat;tabs.querySelectorAll('.ow-chip').forEach(function(c){c.classList.remove('ow-active')});this.classList.add('ow-active');render();}}));
    });
    search.querySelector('input').addEventListener('input', render);
    _append(wrap, search, tabs, grid);
    render();
    return wrap;
  },

  _widgetPreview: function(id) {
    var p = {
      'btn-primary':'<button class="ow-btn ow-btn-primary ow-btn-sm">Primary</button>',
      'btn-solid':  '<button class="ow-btn ow-btn-solid ow-btn-sm">Solid</button>',
      'btn-ghost':  '<button class="ow-btn ow-btn-ghost ow-btn-sm">Ghost</button>',
      'btn-danger': '<button class="ow-btn ow-btn-danger ow-btn-sm">Danger</button>',
      'btn-success':'<button class="ow-btn ow-btn-success ow-btn-sm">Success</button>',
      'btn-fab':    '<button class="ow-btn ow-btn-fab ow-btn-fab-sm">+</button>',
      'btn-group':  '<div class="ow-btn-group"><button class="ow-btn ow-btn-ghost ow-btn-sm">A</button><button class="ow-btn ow-btn-ghost ow-btn-sm">B</button><button class="ow-btn ow-btn-ghost ow-btn-sm">C</button></div>',
      'input-text': '<input class="ow-input ow-input-sm" placeholder="Text input…" style="max-width:140px">',
      'input-toggle':'<label class="ow-toggle-wrap"><div class="ow-toggle on"></div><span>Enabled</span></label>',
      'input-checkbox':'<label class="ow-cb-wrap"><input class="ow-cb" type="checkbox" checked><span>Option</span></label>',
      'input-slider':'<input class="ow-slider" type="range" value="60" style="max-width:120px">',
      'card-basic': '<div class="ow-card" style="min-width:120px;padding:10px"><div style="font-size:10px;font-weight:700">Card</div><div style="font-size:9px;color:var(--ow-text-dim)">Content area</div></div>',
      'card-stat':  '<div class="ow-stat" style="min-width:90px"><div class="ow-stat-val">247</div><div class="ow-stat-label">Requests</div></div>',
      'card-action':'<button class="ow-action" style="min-width:140px"><div class="ow-action-icon">🚀</div><div><div class="ow-action-title" style="font-size:10px">Launch</div></div><span class="ow-action-arrow">›</span></button>',
      'nav-tabs':   '<div class="ow-tabs"><button class="ow-tab ow-active">Tab 1</button><button class="ow-tab">Tab 2</button><button class="ow-tab">Tab 3</button></div>',
      'fb-badge':   '<span class="ow-badge">Active</span> <span class="ow-badge ow-badge-success">OK</span> <span class="ow-badge ow-badge-danger">Error</span>',
      'fb-progress':'<div style="width:120px"><div class="ow-progress"><div class="ow-progress-fill" style="width:72%"></div></div></div>',
      'fb-spinner': '<div class="ow-spin ow-spin-sm"></div>',
      'fb-chip':    '<span class="ow-chip ow-active">🏷 Widget</span>',
      'data-table': '<div style="font-size:9px;color:var(--ow-text-dim)">┌─────┬────────┐<br>│ ID  │ Status │<br>├─────┼────────┤<br>│ 001 │ ✓ OK   │<br>└─────┴────────┘</div>',
      'data-metric':'<div class="ow-metric"><div class="ow-metric-val" style="font-size:22px">98<span class="ow-metric-unit">%</span></div><div class="ow-metric-lbl">Uptime</div></div>',
      'sp-health':  '<svg width="52" height="52" viewBox="0 0 52 52" style="transform:rotate(-90deg)"><circle cx="26" cy="26" r="20" fill="none" stroke="var(--ow-bg-raise)" stroke-width="5"/><circle cx="26" cy="26" r="20" fill="none" stroke="var(--ow-success)" stroke-width="5" stroke-dasharray="125.6" stroke-dashoffset="18" stroke-linecap="round"/></svg>',
      'sp-theme':   '<div style="display:flex;gap:5px">'+OW_THEMES.map(function(t){return '<div style="width:16px;height:16px;border-radius:50%;background:'+t.swatch+'"></div>';}).join('')+'</div>',
    };
    return p[id] || '<div style="font-size:10px;color:var(--ow-text-dim);text-align:center">Preview</div>';
  },

  /* ── Register custom widget ─────────────────────────────────────────────────── */
  registerWidget: function(def) {
    if (!def || !def.id) return;
    OW_WIDGET_DB.push(def);
  },

  /* ── Utility ────────────────────────────────────────────────────────────────── */
  el: _el,
};

  /* ── VS Code IPC bridge ──────────────────────────────────────────────────────── */
  /* Stores vscode webview API so theme changes can broadcast to the extension host */
  _vscodeApi: null,
  _owSyncing: false,

  setVscodeApi: function(api) {
    OW._vscodeApi = api;
  },

};

/* ── Auto-init ───────────────────────────────────────────────────────────────── */
if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', function() { OW.loadTheme(); });
  } else {
    OW.loadTheme();
  }
}

/* ── Theme IPC: receive owThemeSync broadcast from extension host ──────────── */
if (typeof window !== 'undefined') {
  window.addEventListener('message', function(event) {
    var d = event.data;
    if (d && d.type === 'owThemeSync' && d.theme && global.OW) {
      OW._owSyncing = true;
      OW.switchTheme(d.theme);
      OW._owSyncing = false;
      // Persist new theme in VS Code webview state
      if (OW._vscodeApi) {
        try {
          var s = OW._vscodeApi.getState() || {};
          s.owTheme = d.theme;
          OW._vscodeApi.setState(s);
        } catch(e){}
      }
    }
  });
}

global.OW = OW;
})(typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this);
