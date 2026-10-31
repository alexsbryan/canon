#!/usr/bin/env node
import { join } from 'node:path';
import { runHouse } from '../world/house.mjs';
await runHouse(join(import.meta.dirname, 'coliving.json'));
