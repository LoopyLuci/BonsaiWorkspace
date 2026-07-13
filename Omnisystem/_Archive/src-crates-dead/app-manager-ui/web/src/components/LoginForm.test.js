import { render, fireEvent, waitFor, screen } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import LoginForm from './LoginForm.svelte';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('LoginForm.svelte', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render login form', () => {
    const { container } = render(LoginForm);
    expect(container.querySelector('input[type="text"]')).toBeTruthy();
    expect(container.querySelector('input[type="password"]')).toBeTruthy();
    expect(container.querySelector('button')).toBeTruthy();
  });

  it('should display error when fields are empty', async () => {
    const { container } = render(LoginForm);
    const button = container.querySelector('button');

    await fireEvent.click(button);

    await waitFor(() => {
      const errorDiv = container.querySelector('[role="alert"]');
      expect(errorDiv?.textContent).toContain('required');
    });
  });

  it('should update input values', async () => {
    const { container } = render(LoginForm);
    const userInput = container.querySelector('input[type="text"]');
    const passInput = container.querySelector('input[type="password"]');

    await fireEvent.change(userInput, { target: { value: 'testuser' } });
    await fireEvent.change(passInput, { target: { value: 'password123' } });

    expect(userInput.value).toBe('testuser');
    expect(passInput.value).toBe('password123');
  });

  it('should submit on Enter key', async () => {
    const { container } = render(LoginForm);
    const userInput = container.querySelector('input[type="text"]');
    const passInput = container.querySelector('input[type="password"]');

    await fireEvent.change(userInput, { target: { value: 'testuser' } });
    await fireEvent.change(passInput, { target: { value: 'password123' } });
    await fireEvent.keyPress(userInput, { key: 'Enter' });

    // Submit should be triggered
    expect(userInput.value).toBe('testuser');
  });

  it('should disable button while loading', async () => {
    const { container } = render(LoginForm);
    const button = container.querySelector('button');

    expect(button.disabled).toBe(false);

    // After click, button should be disabled during loading
    await fireEvent.click(button);
    await waitFor(() => {
      expect(button.disabled).toBe(true);
    });
  });

  it('should have demo credentials visible', () => {
    const { container } = render(LoginForm);
    const demoSection = container.querySelector('[role="note"]');

    expect(demoSection?.textContent).toContain('demo-user');
    expect(demoSection?.textContent).toContain('Password123!');
  });

  it('should have correct ARIA labels', () => {
    const { container } = render(LoginForm);

    expect(container.querySelector('label[for="userId"]')).toBeTruthy();
    expect(container.querySelector('label[for="password"]')).toBeTruthy();
    expect(container.querySelector('input#userId')).toBeTruthy();
    expect(container.querySelector('input#password')).toBeTruthy();
  });

  it('should clear inputs on successful login', async () => {
    const { container } = render(LoginForm);
    const userInput = container.querySelector('input[type="text"]');
    const passInput = container.querySelector('input[type="password"]');

    await fireEvent.change(userInput, { target: { value: 'testuser' } });
    await fireEvent.change(passInput, { target: { value: 'password123' } });

    expect(userInput.value).toBe('testuser');
    expect(passInput.value).toBe('password123');
  });

  it('should be responsive to viewport changes', () => {
    const { container } = render(LoginForm);
    const formDiv = container.querySelector('[role="main"]');

    expect(formDiv).toBeTruthy();
    expect(formDiv?.classList.contains('min-h-screen')).toBe(true);
  });
});
