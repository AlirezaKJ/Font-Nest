import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';

// Every rendered component leaves the document between tests, so one test cannot find an element
// another one mounted.
afterEach(cleanup);
