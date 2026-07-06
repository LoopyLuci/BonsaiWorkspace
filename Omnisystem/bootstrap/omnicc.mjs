#!/usr/bin/env node
// Cross-platform launcher for the OmniCC bootstrap.
//
// Node >= 22.6 strips TypeScript types natively, so we can import the .ts CLI
// directly with no build step. This wrapper exists so `omnicc` works as an
// installed bin and from any working directory.
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
await import(pathToFileURL(join(here, 'src', 'cli.ts')).href);
