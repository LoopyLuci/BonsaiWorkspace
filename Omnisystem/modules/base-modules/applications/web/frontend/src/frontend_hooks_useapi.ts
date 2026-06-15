// PATHFINDER Frontend - useApi Hook
// Custom hook for API calls with error handling, retry logic, and caching

import { useState, useCallback } from 'react';
import apiClient from '../api-client';

interface UseApiOptions {
  retry?: number;
  cache?: boolean;
  cacheDuration?: number; // milliseconds
}

interface CacheEntry {
  data: any;
  timestamp: number;
}

const cache = new Map<string, CacheEntry>();

interface UseApiReturn<T> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
  execute: (fn: () => Promise<T>) => Promise<T>;
}

export const useApi = <T = any>(options: UseApiOptions = {}): UseApiReturn<T> => {
  const {
    retry = 3,
    cache: useCache = true,
    cacheDuration = 5 * 60 * 1000, // 5 minutes default
  } = options;

  const [data, setData] = useState<T | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(
    async (fn: () => Promise<T>): Promise<T> => {
      // Generate cache key from function
      const cacheKey = fn.toString();

      // Check cache
      if (useCache && cache.has(cacheKey)) {
        const entry = cache.get(cacheKey)!;
        if (Date.now() - entry.timestamp < cacheDuration) {
          setData(entry.data);
          return entry.data;
        }
      }

      setIsLoading(true);
      setError(null);

      let lastError: Error | null = null;
      let result: T | null = null;

      // Retry loop
      for (let attempt = 0; attempt < retry; attempt++) {
        try {
          result = await fn();
          setData(result);
          setError(null);

          // Cache result
          if (useCache) {
            cache.set(cacheKey, {
              data: result,
              timestamp: Date.now(),
            });
          }

          setIsLoading(false);
          return result;
        } catch (err: any) {
          lastError = err;

          // Don't retry on 4xx errors (bad request, not found, etc.)
          if (err.response?.status >= 400 && err.response?.status < 500) {
            break;
          }

          // Wait before retrying (exponential backoff)
          if (attempt < retry - 1) {
            await new Promise((resolve) =>
              setTimeout(resolve, Math.pow(2, attempt) * 1000)
            );
          }
        }
      }

      // All retries failed
      const errorMessage =
        lastError?.response?.data?.message ||
        lastError?.message ||
        'Failed to fetch data';

      setError(errorMessage);
      setIsLoading(false);
      throw lastError;
    },
    [retry, useCache, cacheDuration]
  );

  return {
    data,
    isLoading,
    error,
    execute,
  };
};

// Specific hook for fetching (GET requests)
export const useFetch = <T = any>(
  url: string,
  options: UseApiOptions = {}
): UseApiReturn<T> => {
  const api = useApi<T>(options);

  const fetchData = useCallback(
    () => apiClient.get<T>(url),
    [url]
  );

  return {
    ...api,
    execute: fetchData as any,
  };
};

// Specific hook for mutations (POST, PUT, DELETE)
interface UseMutationReturn<T> {
  isLoading: boolean;
  error: string | null;
  execute: (data?: any) => Promise<T>;
}

export const useMutation = <T = any>(
  fn: (data?: any) => Promise<T>,
  options: UseApiOptions = {}
): UseMutationReturn<T> => {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(
    async (data?: any) => {
      setIsLoading(true);
      setError(null);

      try {
        const result = await fn(data);
        setIsLoading(false);
        return result;
      } catch (err: any) {
        const errorMessage =
          err.response?.data?.message || err.message || 'Operation failed';
        setError(errorMessage);
        setIsLoading(false);
        throw err;
      }
    },
    [fn]
  );

  return {
    isLoading,
    error,
    execute,
  };
};
