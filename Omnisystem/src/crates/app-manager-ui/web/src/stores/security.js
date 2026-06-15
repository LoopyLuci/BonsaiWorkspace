/**
 * Security & Hardening Module
 * Provides XSS prevention, input validation, CSRF protection, and secure storage
 */

import crypto from 'crypto';

// CSRF Token Management
let csrfToken = null;

export function generateCSRFToken() {
  csrfToken = crypto.randomBytes(32).toString('hex');
  localStorage.setItem('csrf_token', csrfToken);
  return csrfToken;
}

export function getCSRFToken() {
  if (!csrfToken) {
    csrfToken = localStorage.getItem('csrf_token');
    if (!csrfToken) {
      csrfToken = generateCSRFToken();
    }
  }
  return csrfToken;
}

export function validateCSRFToken(token) {
  return token === getCSRFToken();
}

// Input Sanitization
const DANGEROUS_PATTERNS = [
  /<script[^>]*>[\s\S]*?<\/script>/gi,
  /javascript:/gi,
  /on\w+\s*=/gi,
  /<iframe/gi,
  /<embed/gi,
  /<object/gi,
];

export function sanitizeInput(input) {
  if (typeof input !== 'string') {
    return input;
  }

  let sanitized = input
    .trim()
    .replace(/[<>]/g, '')
    .replace(/javascript:/gi, '');

  for (const pattern of DANGEROUS_PATTERNS) {
    sanitized = sanitized.replace(pattern, '');
  }

  return sanitized;
}

export function sanitizeObject(obj) {
  if (typeof obj !== 'object' || obj === null) {
    return sanitizeInput(obj);
  }

  if (Array.isArray(obj)) {
    return obj.map(item => sanitizeObject(item));
  }

  const sanitized = {};
  for (const [key, value] of Object.entries(obj)) {
    sanitized[key] = sanitizeObject(value);
  }
  return sanitized;
}

// Content Security Policy Helpers
export const CSP_HEADERS = {
  'Content-Security-Policy': `
    default-src 'self';
    script-src 'self' 'unsafe-inline' 'unsafe-eval' data:;
    style-src 'self' 'unsafe-inline';
    img-src 'self' data: https:;
    font-src 'self' data:;
    connect-src 'self' http://localhost:* https:;
    frame-ancestors 'none';
    base-uri 'self';
    form-action 'self';
  `.replace(/\s+/g, ' ').trim(),
};

// Secure Token Storage
class SecureTokenStore {
  constructor() {
    this.memoryStore = new Map();
  }

  setToken(key, value) {
    // Store in memory (session-only)
    this.memoryStore.set(key, {
      value,
      timestamp: Date.now(),
      expiresIn: 3600000, // 1 hour
    });
  }

  getToken(key) {
    const entry = this.memoryStore.get(key);
    if (!entry) return null;

    if (Date.now() - entry.timestamp > entry.expiresIn) {
      this.memoryStore.delete(key);
      return null;
    }

    return entry.value;
  }

  clearToken(key) {
    this.memoryStore.delete(key);
  }

  clearAll() {
    this.memoryStore.clear();
  }

  isExpired(key) {
    const entry = this.memoryStore.get(key);
    if (!entry) return true;
    return Date.now() - entry.timestamp > entry.expiresIn;
  }
}

export const tokenStore = new SecureTokenStore();

// Input Validation
export const validators = {
  email: (email) => {
    const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return regex.test(email) && email.length <= 254;
  },

  password: (password) => {
    // Minimum 8 chars, 1 uppercase, 1 lowercase, 1 number, 1 special char
    const regex = /^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$/;
    return regex.test(password);
  },

  username: (username) => {
    // Alphanumeric, underscore, hyphen, 3-20 chars
    const regex = /^[a-zA-Z0-9_-]{3,20}$/;
    return regex.test(username);
  },

  uuid: (uuid) => {
    const regex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
    return regex.test(uuid);
  },

  url: (url) => {
    try {
      new URL(url);
      return true;
    } catch {
      return false;
    }
  },

  alphanumeric: (str) => /^[a-zA-Z0-9]+$/.test(str),

  noSql: (input) => {
    // Check for NoSQL injection patterns
    const dangerous = ['{', '}', '$', 'function', 'constructor'];
    const lowerInput = input.toLowerCase();
    return !dangerous.some(pattern => lowerInput.includes(pattern));
  },
};

// Rate Limiting for Client-Side
export class ClientRateLimiter {
  constructor(maxRequests = 100, windowMs = 60000) {
    this.maxRequests = maxRequests;
    this.windowMs = windowMs;
    this.requests = [];
  }

  isAllowed() {
    const now = Date.now();
    // Remove old requests outside window
    this.requests = this.requests.filter(time => now - time < this.windowMs);

    if (this.requests.length < this.maxRequests) {
      this.requests.push(now);
      return true;
    }

    return false;
  }

  reset() {
    this.requests = [];
  }

  getRemainingRequests() {
    const now = Date.now();
    const validRequests = this.requests.filter(time => now - time < this.windowMs);
    return Math.max(0, this.maxRequests - validRequests.length);
  }
}

// Secure API Request Wrapper
export async function secureRequest(method, endpoint, data = null) {
  const csrfToken = getCSRFToken();
  const sanitized = data ? sanitizeObject(data) : null;

  const headers = {
    'Content-Type': 'application/json',
    'X-CSRF-Token': csrfToken,
    'X-Requested-With': 'XMLHttpRequest',
  };

  const options = {
    method,
    headers,
  };

  if (sanitized) {
    options.body = JSON.stringify(sanitized);
  }

  try {
    const response = await fetch(endpoint, options);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json();
  } catch (error) {
    console.error('Secure request failed:', error);
    throw error;
  }
}

// Security Headers Check
export function checkSecurityHeaders(response) {
  const securityHeaders = [
    'Content-Security-Policy',
    'X-Content-Type-Options',
    'X-Frame-Options',
    'X-XSS-Protection',
    'Strict-Transport-Security',
  ];

  const missing = [];
  for (const header of securityHeaders) {
    if (!response.headers.get(header)) {
      missing.push(header);
    }
  }

  return {
    hasAllHeaders: missing.length === 0,
    missingHeaders: missing,
  };
}

// Sensitive Data Mask
export function maskSensitiveData(data, fieldsToMask = ['password', 'token', 'secret']) {
  if (typeof data !== 'object' || data === null) {
    return data;
  }

  const masked = { ...data };
  for (const field of fieldsToMask) {
    if (field in masked) {
      const value = String(masked[field]);
      masked[field] = value.substring(0, 2) + '*'.repeat(Math.max(0, value.length - 4)) + value.substring(Math.max(0, value.length - 2));
    }
  }

  return masked;
}

// Audit Logging
export class AuditLogger {
  constructor() {
    this.logs = [];
  }

  log(action, details, severity = 'info') {
    const entry = {
      timestamp: new Date().toISOString(),
      action,
      details: maskSensitiveData(details),
      severity,
    };

    this.logs.push(entry);

    // Keep only last 1000 logs in memory
    if (this.logs.length > 1000) {
      this.logs.shift();
    }

    if (severity === 'error' || severity === 'critical') {
      console.error(`[${severity.toUpperCase()}] ${action}:`, details);
    }
  }

  getLogs() {
    return [...this.logs];
  }

  getLogsBySeverity(severity) {
    return this.logs.filter(log => log.severity === severity);
  }

  clear() {
    this.logs = [];
  }
}

export const auditLogger = new AuditLogger();

// Initialize security on load
export function initializeSecurity() {
  generateCSRFToken();
  auditLogger.log('security_initialized', { timestamp: Date.now() }, 'info');
}
